//! Line and position helpers. LSP positions are UTF-16 based, the compiler works on bytes.

use lsp_types::{Position, Range};

/// Splits a document into lines the same way the compiler does (`str::lines`), but keeps a
/// trailing empty line so that the cursor on the last line of a file is always addressable.
pub fn split_lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.lines().collect();
    if text.is_empty() || text.ends_with('\n') {
        lines.push("");
    }
    lines
}

/// Everything before the first `;` (comments are stripped by the compiler the same way).
pub fn code_part(line: &str) -> &str {
    line.split(';').next().unwrap_or_default()
}

pub fn is_indented(line: &str) -> bool {
    line.starts_with(' ') || line.starts_with('\t')
}

/// Byte offset → UTF-16 column.
pub fn utf16_col(line: &str, byte: usize) -> u32 {
    line[..byte.min(line.len())].encode_utf16().count() as u32
}

/// UTF-16 column → byte offset (clamped to the line).
pub fn byte_of_col(line: &str, col: u32) -> usize {
    let mut units = 0u32;
    for (i, ch) in line.char_indices() {
        if units >= col {
            return i;
        }
        units += ch.len_utf16() as u32;
    }
    line.len()
}

pub fn range(line_idx: usize, line: &str, start: usize, end: usize) -> Range {
    Range::new(
        Position::new(line_idx as u32, utf16_col(line, start)),
        Position::new(line_idx as u32, utf16_col(line, end)),
    )
}

/// Byte span of the code on a line without leading/trailing whitespace (and without comments).
pub fn trimmed_span(line: &str) -> (usize, usize) {
    let code = code_part(line);
    let start = code.len() - code.trim_start().len();
    let end = code.trim_end().len();
    if end <= start { (0, 0) } else { (start, end) }
}

/// Trims a sub-slice `s[start..end]` and returns the trimmed byte span.
pub fn trim_span(s: &str, start: usize, end: usize) -> (usize, usize) {
    let sub = &s[start..end];
    let lead = sub.len() - sub.trim_start().len();
    let trail = sub.len() - sub.trim_end().len();
    if lead + trail >= sub.len() {
        (start + lead.min(sub.len()), start + lead.min(sub.len()))
    } else {
        (start + lead, end - trail)
    }
}

/// Name of the routine that contains `line_idx` (the closest unindented `name:` line above it).
pub fn enclosing_routine(lines: &[&str], line_idx: usize) -> Option<String> {
    for line in lines[..=line_idx.min(lines.len().saturating_sub(1))]
        .iter()
        .rev()
    {
        let code = code_part(line);
        if code.trim().is_empty() || is_indented(code) {
            continue;
        }
        let trimmed = code.trim();
        if let Some(name) = trimmed.strip_suffix(':')
            && !name.contains(' ')
        {
            return Some(name.to_string());
        }
        return None;
    }
    None
}

/// Path for display, without the `\\?\` prefix that `canonicalize` adds on Windows.
pub fn display_path(p: &std::path::Path) -> String {
    let s = p.display().to_string();
    s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
}

/// Shortens long values for display in completion rows.
pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans() {
        assert_eq!(trimmed_span("    add c1, 2   ; comment"), (4, 13));
        assert_eq!(trimmed_span("   "), (0, 0));
        assert_eq!(trim_span("a,  b  ,c", 2, 7), (4, 5));
    }

    #[test]
    fn utf16() {
        let l = "  é😀x";
        let b = l.find('x').unwrap();
        assert_eq!(utf16_col(l, b), 5);
        assert_eq!(byte_of_col(l, 5), b);
    }

    #[test]
    fn routines() {
        let lines = vec![
            "_init:",
            "    alias a, 1",
            "; comment",
            "main:",
            "    add c1, 1",
            "",
        ];
        assert_eq!(enclosing_routine(&lines, 1).as_deref(), Some("_init"));
        assert_eq!(enclosing_routine(&lines, 4).as_deref(), Some("main"));
        assert_eq!(enclosing_routine(&lines, 5).as_deref(), Some("main"));
    }
}
