//! Splits an instruction line into mnemonic, arguments and flags (byte spans), following
//! `Tasm::parse_instr_line`: flags come after the `|`, the mnemonic ends at the first space and
//! arguments are separated by commas.

use crate::{
    index::split_args,
    text::{code_part, is_indented},
};

pub struct InstrLine {
    /// Span of the mnemonic, without the `~` concurrency prefix.
    pub mnemonic: (usize, usize),
    /// Trimmed argument spans and their text. Empty if the line has no argument region.
    pub args: Vec<((usize, usize), String)>,
    /// Region after the `|`, up to the comment.
    pub flags: Option<(usize, usize)>,
}

impl InstrLine {
    pub fn mnemonic_text<'a>(&self, line: &'a str) -> &'a str {
        &line[self.mnemonic.0..self.mnemonic.1]
    }
}

/// Works on full lines and on the part of a line before the cursor.
pub fn parse_instr_line(line: &str) -> Option<InstrLine> {
    let code = code_part(line);
    if !is_indented(code) {
        return None;
    }
    let mut start = code.len() - code.trim_start().len();
    if code[start..].starts_with('~') {
        start += 1;
    }
    let end = code[start..]
        .find(|c: char| c.is_whitespace() || c == '|')
        .map(|i| start + i)
        .unwrap_or(code.len());

    let pipe = code.find('|');
    let args_end = pipe.unwrap_or(code.len()).max(end);
    let args = if end < args_end && !code[end..args_end].trim().is_empty() {
        split_args(line, end, args_end)
    } else if end < args_end {
        // only whitespace after the mnemonic: an empty first argument at the end
        vec![((args_end, args_end), String::new())]
    } else {
        vec![]
    };

    Some(InstrLine {
        mnemonic: (start, end),
        args,
        flags: pipe.map(|p| (p + 1, code.len())),
    })
}

/// The flag-region word around `col`. Words are separated by whitespace and dict punctuation,
/// so `remap:{1=a}` yields `remap:`, `1`, and `a`. Returns (span, inside a dict).
pub fn flag_word(line: &str, region: (usize, usize), col: usize) -> Option<((usize, usize), bool)> {
    let is_delim = |c: char| c.is_whitespace() || matches!(c, '{' | '}' | ',' | '=');
    let text = &line[region.0..region.1];
    let rel = col.checked_sub(region.0)?.min(text.len());
    let start = text[..rel]
        .rfind(is_delim)
        .map(|i| i + text[i..].chars().next().map_or(1, char::len_utf8))
        .unwrap_or(0);
    let end = text[rel..]
        .find(is_delim)
        .map(|i| rel + i)
        .unwrap_or(text.len());
    if start >= end {
        return None;
    }
    let before = &text[..start];
    let in_dict = before.matches('{').count() > before.matches('}').count();
    Some(((region.0 + start, region.0 + end), in_dict))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts() {
        let l = "    ~add c1, n | itemmod:2 ; c";
        let p = parse_instr_line(l).unwrap();
        assert_eq!(p.mnemonic_text(l), "add");
        assert_eq!(
            p.args.iter().map(|a| a.1.as_str()).collect::<Vec<_>>(),
            ["c1", "n"]
        );
        assert_eq!(&l[p.flags.unwrap().0..p.flags.unwrap().1], " itemmod:2 ");

        let l = "    nop";
        let p = parse_instr_line(l).unwrap();
        assert!(p.args.is_empty() && p.flags.is_none());

        let l = "    add ";
        let p = parse_instr_line(l).unwrap();
        assert_eq!(p.args.len(), 1);
        assert_eq!(p.args[0].0, (8, 8));

        assert!(parse_instr_line("main:").is_none());
    }

    #[test]
    fn words() {
        let l = "  spawn g1 | remap:{ a = mod::b } delay:1";
        let p = parse_instr_line(l).unwrap();
        let region = p.flags.unwrap();
        let (span, in_dict) = flag_word(l, region, l.find("mod").unwrap() + 2).unwrap();
        assert_eq!(&l[span.0..span.1], "mod::b");
        assert!(in_dict);
        let (span, in_dict) = flag_word(l, region, l.find("delay").unwrap()).unwrap();
        assert_eq!(&l[span.0..span.1], "delay:1");
        assert!(!in_dict);
    }
}
