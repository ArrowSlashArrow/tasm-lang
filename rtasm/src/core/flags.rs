use std::collections::HashMap;

use gdlib::gdobj::triggers::{Op, RoundMode, SignMode};

use crate::core::structs::{SymbolPath, split_at_str_once};

#[derive(Debug, Clone)]
pub struct Flag {
    pub ident: String,
    pub value: FlagValue,
    pub ftype: FlagValueType,
}

impl Flag {
    pub fn from(
        ident: String,
        val: &str,
        t: FlagValueType,
        gm: &HashMap<String, i16>,
        aliases: &HashMap<String, String>,
    ) -> Option<Self> {
        Some(Self {
            value: FlagValue::try_from(val, &t, gm, aliases)?,
            ident,
            ftype: t,
        })
    }
}

#[derive(Debug, Clone)]
pub enum FlagValue {
    RoundSign((RoundMode, SignMode)),
    Float(f64),
    Op(Op),
    Dict(Vec<(i16, i16)>),
    Bool(bool),
    // parse this to a dict again when we resolve the external references
    ExternRefsDict(
        (
            Vec<(i16, i16)>,
            Vec<(UnparsedDictFlagEntry, UnparsedDictFlagEntry)>,
        ),
    ),
}

#[derive(Debug, Clone)]
pub enum FlagValueType {
    RoundSign,
    Float,
    Op,
    Dict,
    Bool,
}

fn string_to_roundsign(s: &str) -> FlagValue {
    if s.is_empty() {
        return FlagValue::RoundSign((RoundMode::None, SignMode::None));
    }
    /* values have to be formatted like {round}{sign}:
     * round+   r+
     * round-   r-
     * round    r
     * floor+   f+
     * floor-   f-
     * floor    f
     * ceil+    c+
     * ceil-    c-
     * ceil     c
     * +        +
     * -        -
     *
     * if the roundmode is not a recognized string, it defaults to none.
     */

    let suf = s.chars().last().unwrap();
    let mut pref = s.to_owned();

    let sign = match suf {
        '+' => {
            pref.pop();
            SignMode::Absolute
        }
        '-' => {
            pref.pop();
            SignMode::Negative
        }
        _ => SignMode::None,
    };

    let round = match pref.as_str() {
        "round" | "r" => RoundMode::Nearest,
        "ceil" | "c" => RoundMode::Ceiling,
        "floor" | "f" => RoundMode::Floor,
        _ => RoundMode::None,
    };

    FlagValue::RoundSign((round, sign))
}

impl From<FlagValue> for f64 {
    fn from(val: FlagValue) -> Self {
        val.to_float().unwrap()
    }
}
impl From<FlagValue> for bool {
    fn from(val: FlagValue) -> Self {
        val.to_bool().unwrap()
    }
}
impl From<FlagValue> for Op {
    fn from(val: FlagValue) -> Self {
        val.to_op().unwrap()
    }
}
impl From<FlagValue> for Vec<(i16, i16)> {
    fn from(val: FlagValue) -> Self {
        val.to_cloned_dict().unwrap()
    }
}
impl From<FlagValue> for (RoundMode, SignMode) {
    fn from(val: FlagValue) -> Self {
        val.to_roundsign().unwrap()
    }
}

impl FlagValue {
    fn try_from(
        value: &str,
        t: &FlagValueType,
        group_map: &HashMap<String, i16>,
        aliases: &HashMap<String, String>,
    ) -> Option<Self> {
        match t {
            FlagValueType::RoundSign => Some(string_to_roundsign(value)),
            FlagValueType::Float => match value.parse::<f64>() {
                Ok(f) => {
                    if f.is_finite() {
                        Some(Self::Float(f))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            },
            FlagValueType::Op => match value {
                "+" => Some(Self::Op(Op::Add)),
                "-" => Some(Self::Op(Op::Sub)),
                "*" => Some(Self::Op(Op::Mul)),
                "/" => Some(Self::Op(Op::Div)),
                _ => None,
            },
            FlagValueType::Dict => {
                let mut invalid_dict = false;
                let mut has_extern_refs = false;
                let resolve_int = |s: &str| -> Option<i16> {
                    s.parse::<i16>().ok().or_else(|| group_map.get(s).copied())
                };

                let parse_int = |s: &str, invalid_dict: &mut bool| -> i16 {
                    resolve_int(s)
                        .or_else(|| aliases.get(s).and_then(|a| resolve_int(a)))
                        .unwrap_or_else(|| {
                            *invalid_dict = true;
                            0
                        })
                };

                let mut unparsed_pairs: Vec<(UnparsedDictFlagEntry, UnparsedDictFlagEntry)> =
                    vec![];

                let resolve_unparsed_entry =
                    |s: &str, invalid_dict: &mut bool| -> UnparsedDictFlagEntry {
                        match split_at_str_once(s, "::") {
                            Some((left, right)) => {
                                UnparsedDictFlagEntry::Path(SymbolPath {
                                    root: Some(left.to_owned()),
                                    ident: right.to_owned(),
                                    assigned_group: -1, // this will get filled in during post-linking
                                })
                            }
                            None => UnparsedDictFlagEntry::Int(parse_int(s, invalid_dict)),
                        }
                    };

                let kv_pairs: Vec<(i16, i16)> = value[1..value.len() - 1]
                    .split(',')
                    .map(|kv| {
                        let (key, v) = match split_at_str_once(kv, "=") {
                            Some(p) => p,
                            None => {
                                invalid_dict = true;
                                ("", "")
                            }
                        };

                        if key.contains("::") || v.contains("::") {
                            let key_parsed = resolve_unparsed_entry(key, &mut invalid_dict);
                            let value_parsed = resolve_unparsed_entry(v, &mut invalid_dict);

                            has_extern_refs = true;
                            unparsed_pairs.push((key_parsed, value_parsed));
                            (0, 0)
                        } else {
                            (
                                parse_int(key, &mut invalid_dict),
                                parse_int(v, &mut invalid_dict),
                            )
                        }
                    })
                    .collect::<Vec<_>>();

                if invalid_dict {
                    None
                } else if has_extern_refs {
                    Some(Self::ExternRefsDict((kv_pairs, unparsed_pairs)))
                } else {
                    Some(Self::Dict(kv_pairs))
                }
            }
            FlagValueType::Bool => match value {
                "true" => Some(Self::Bool(true)),
                "false" => Some(Self::Bool(false)),
                _ => None,
            },
        }
    }

    pub fn get_type(&self) -> FlagValueType {
        match self {
            Self::Bool(_) => FlagValueType::Bool,
            Self::Dict(_) => FlagValueType::Dict,
            Self::Op(_) => FlagValueType::Op,
            Self::Float(_) => FlagValueType::Float,
            Self::RoundSign(_) => FlagValueType::RoundSign,
            Self::ExternRefsDict(_) => FlagValueType::Dict,
        }
    }

    pub fn to_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn to_dict(&self) -> Option<&Vec<(i16, i16)>> {
        match self {
            Self::Dict(d) => Some(d),
            _ => None,
        }
    }
    pub fn to_cloned_dict(&self) -> Option<Vec<(i16, i16)>> {
        match self {
            Self::Dict(d) => Some(d.clone()),
            _ => None,
        }
    }
    pub fn to_owned_dict(self) -> Option<Vec<(i16, i16)>> {
        match self {
            Self::Dict(d) => Some(d),
            _ => None,
        }
    }
    pub fn to_roundsign(&self) -> Option<(RoundMode, SignMode)> {
        match self {
            Self::RoundSign(f) => Some(*f),
            _ => None,
        }
    }
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(f) => Some(*f),
            _ => None,
        }
    }
    pub fn to_op(&self) -> Option<Op> {
        match self {
            Self::Op(f) => Some(*f),
            _ => None,
        }
    }
}

pub fn get_flag_type(ident: &str) -> Option<FlagValueType> {
    Some(match ident {
        "resmode" => FlagValueType::RoundSign,
        "finmode" => FlagValueType::RoundSign,
        "itemmod" => FlagValueType::Float,
        "divmod" => FlagValueType::Bool,
        "iter" => FlagValueType::Op,
        "op" => FlagValueType::Op,
        "delay" => FlagValueType::Float,
        "remap" => FlagValueType::Dict,
        "ordered" => FlagValueType::Bool,
        "noremap" => FlagValueType::Bool,
        "tpaused" => FlagValueType::Bool,
        "tmod" => FlagValueType::Float,
        "tstop" => FlagValueType::Bool,
        "nover" => FlagValueType::Bool,
        _ => return None,
    })
}

#[derive(Debug, Clone)]
pub enum UnparsedDictFlagEntry {
    Int(i16),
    Path(SymbolPath),
}
