// todo: replace with calls to rtasm

#[derive(Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

pub struct Diag {
    pub line: u32,
    pub start: u32,
    pub end: u32,
    pub message: String,
    pub severity: Severity,
}

pub struct Routine {
    pub name: String,
    pub line: u32,
    pub name_end: u32,
    pub last_line: u32,
    pub last_col: u32,
}

#[derive(Default)]
pub struct Analysis {
    pub routines: Vec<Routine>,
    pub aliases: Vec<String>,
    pub diagnostics: Vec<Diag>,
}

const INIT_ONLY: &[&str] = &[
    "alias",
    "import",
    "importstd",
    "display",
    "ioblock",
    "coll",
    "event",
];
const ROUTINE_ARG: &[&str] = &["spawn", "pause", "resume", "kill"];
const BUILTINS: &[&str] = &[
    "ATTEMPTS",
    "POINTS",
    "MAINTIME",
    "COLL_P1",
    "COLL_P2",
    "COLL_P_ANY",
];

/// Strip a trailing `; comment`. RAW/RAWTRG payloads also end at `;`.
fn code_part(line: &str) -> &str {
    match line.find(';') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
        && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// `g123`, `gx56`
fn is_group_literal(s: &str) -> bool {
    let rest = match s.strip_prefix(['g', 'G']) {
        Some(r) => r,
        None => return false,
    };
    match rest.strip_prefix(['x', 'X']) {
        Some(h) => !h.is_empty() && h.chars().all(|c| c.is_ascii_hexdigit()),
        None => !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()),
    }
}

pub fn analyze(src: &str) -> Analysis {
    let mut a = Analysis::default();
    let lines: Vec<&str> = src.lines().collect();

    // Pass 1: routines and aliases, so forward references resolve.
    for (i, raw) in lines.iter().enumerate() {
        let code = code_part(raw).trim_end();
        if code.is_empty() {
            continue;
        }
        let indented = raw.starts_with([' ', '\t']);
        if !indented {
            if let Some(name) = code.strip_suffix(':') {
                let name = name.trim_end();
                if is_ident(name) {
                    a.routines.push(Routine {
                        name: name.to_string(),
                        line: i as u32,
                        name_end: name.len() as u32,
                        last_line: i as u32,
                        last_col: raw.len() as u32,
                    });
                }
            }
        } else {
            if let Some(r) = a.routines.last_mut() {
                r.last_line = i as u32;
                r.last_col = raw.len() as u32;
            }
            let mut words = code.trim_start().trim_start_matches('~').split_whitespace();
            if words.next().map(|w| w.eq_ignore_ascii_case("alias")) == Some(true) {
                if let Some(n) = words.next() {
                    a.aliases.push(n.trim_end_matches(',').to_string());
                }
            }
        }
    }

    // Pass 2: per-instruction checks.
    let mut current = String::new();
    for (i, raw) in lines.iter().enumerate() {
        let line = i as u32;
        let code = code_part(raw);
        if code.trim().is_empty() {
            continue;
        }
        if !raw.starts_with([' ', '\t']) {
            current = code.trim_end().trim_end_matches(':').to_string();
            continue;
        }
        let lead = raw.len() - raw.trim_start().len();
        let body = code[lead..].trim_end();
        let body_nt = body.strip_prefix('~').unwrap_or(body);
        let tilde = (body.len() - body_nt.len()) as u32;
        let mnemonic = body_nt.split_whitespace().next().unwrap_or("");
        let m_start = lead as u32 + tilde;
        let m_end = m_start + mnemonic.len() as u32;
        let lower = mnemonic.to_ascii_lowercase();

        if INIT_ONLY.contains(&lower.as_str()) && current != "_init" {
            a.diagnostics.push(Diag {
                line,
                start: m_start,
                end: m_end,
                message: format!(
                    "{} is only allowed in the _init routine",
                    mnemonic.to_uppercase()
                ),
                severity: Severity::Error,
            });
        }

        let is_raw = lower == "raw" || lower == "rawtrg";
        if !is_raw && body.matches('|').count() > 1 {
            a.diagnostics.push(Diag {
                line,
                start: lead as u32,
                end: raw.len().min(lead + body.len()) as u32,
                message: "only one `|` is allowed per line".into(),
                severity: Severity::Error,
            });
        }

        if ROUTINE_ARG.contains(&lower.as_str()) {
            let args = body_nt[mnemonic.len()..].split('|').next().unwrap_or("");
            let target = args.split(',').next().unwrap_or("").trim();
            let known = a.routines.iter().any(|r| r.name == target)
                || a.aliases.iter().any(|x| x == target)
                || BUILTINS.contains(&target)
                || target.contains("::")
                || is_group_literal(target);
            if is_ident(target) && !known {
                let col = raw.find(target).unwrap_or(0) as u32;
                a.diagnostics.push(Diag {
                    line,
                    start: col,
                    end: col + target.len() as u32,
                    message: format!("unknown routine `{target}`"),
                    severity: Severity::Warning,
                });
            }
        }
    }
    a
}
