use gdlib::gdobj::triggers::{Op, RoundMode, SignMode};

#[derive(Debug, Clone)]
pub struct Flag {
    pub ident: String,
    pub value: FlagValue,
    pub _type: FlagValueType,
}

impl Flag {
    pub fn from(ident: String, val: &str, t: FlagValueType) -> Option<Self> {
        let value = FlagValue::try_from(val, &t)?;

        Some(Self {
            value,
            ident,
            _type: t,
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
        val.to_dict().unwrap()
    }
}
impl From<FlagValue> for (RoundMode, SignMode) {
    fn from(val: FlagValue) -> Self {
        val.to_roundsign().unwrap()
    }
}

impl FlagValue {
    fn try_from(value: &str, t: &FlagValueType) -> Option<Self> {
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
                let kv_pairs = &value[1..value.len() - 1]
                    .split(',')
                    .map(|kv| {
                        let mut split = kv.trim().split(':');
                        let key = match split.next().unwrap().parse::<i16>() {
                            Ok(n) => n,
                            Err(_) => {
                                invalid_dict = true;
                                0
                            }
                        };
                        let value = match split.next() {
                            Some(v) => match v.parse::<i16>() {
                                Ok(n) => n,
                                Err(_) => {
                                    invalid_dict = true;
                                    0
                                }
                            },
                            None => {
                                invalid_dict = true;
                                0
                            }
                        };
                        (key, value)
                    })
                    .collect::<Vec<_>>();

                if invalid_dict {
                    None
                } else {
                    Some(Self::Dict(kv_pairs.to_owned()))
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
        }
    }

    pub fn to_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn to_dict(&self) -> Option<Vec<(i16, i16)>> {
        match self {
            Self::Dict(d) => Some(d.clone()),
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
        "tpaused" => FlagValueType::Bool,
        "tmod" => FlagValueType::Float,
        "tstop" => FlagValueType::Bool,
        "nover" => FlagValueType::Bool,
        _ => return None,
    })
}
