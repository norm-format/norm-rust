use crate::error::NormError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cell {
    Quoted(String),
    Unquoted(String),
}

impl Cell {
    pub(crate) fn is_empty_unquoted(&self) -> bool {
        matches!(self, Cell::Unquoted(s) if s.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    RootDeclaration { array: bool },
    SectionHeader { name: String, array: bool },
    DataRow { cells: Vec<Cell> },
    Blank,
    Comment,
}

pub(crate) fn lex(input: &str) -> Result<Vec<(usize, Token)>, NormError> {
    if input.starts_with('\u{feff}') {
        return Err(NormError::BomDetected);
    }
    let bytes = input.as_bytes();
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(NormError::BomDetected);
    }

    let mut tokens: Vec<(usize, Token)> = Vec::new();
    let mut pos = 0usize;
    let mut line = 1usize;

    while pos < bytes.len() {
        let line_start = pos;
        let (eol, has_lf) = find_lf(bytes, pos);

        for &b in &bytes[line_start..eol] {
            if b == 0 {
                return Err(NormError::NullByte { line });
            }
        }

        let physical = if eol > line_start && bytes[eol - 1] == b'\r' {
            &bytes[line_start..eol - 1]
        } else {
            &bytes[line_start..eol]
        };

        let leading = trim_start_ws(physical);

        if leading.is_empty() {
            tokens.push((line, Token::Blank));
            pos = if has_lf { eol + 1 } else { eol };
            line += 1;
        } else if leading[0] == b'#' {
            tokens.push((line, Token::Comment));
            pos = if has_lf { eol + 1 } else { eol };
            line += 1;
        } else if leading[0] == b':' {
            let stripped = strip_inline_comment(physical);
            let token = classify_structural(stripped, line)?;
            tokens.push((line, token));
            pos = if has_lf { eol + 1 } else { eol };
            line += 1;
        } else {
            let (cells, new_pos, new_line) = parse_record(bytes, line_start, line)?;
            tokens.push((line, Token::DataRow { cells }));
            pos = new_pos;
            line = new_line;
        }
    }

    Ok(tokens)
}

fn find_lf(bytes: &[u8], start: usize) -> (usize, bool) {
    let mut j = start;
    while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
    }
    (j, j < bytes.len())
}

fn trim_start_ws(s: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
        i += 1;
    }
    &s[i..]
}

fn trim_ws(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t') {
        b -= 1;
    }
    &s[a..b]
}

fn strip_inline_comment(s: &[u8]) -> &[u8] {
    for (i, &b) in s.iter().enumerate() {
        if b == b'#' {
            return &s[..i];
        }
    }
    s
}

fn classify_structural(s: &[u8], line: usize) -> Result<Token, NormError> {
    let t = trim_ws(s);
    if t.first() != Some(&b':') {
        return Err(NormError::InvalidRootDeclaration { line });
    }
    let rest = trim_ws(&t[1..]);

    if rest == b"root" {
        return Ok(Token::RootDeclaration { array: false });
    }
    if rest == b"root[]" {
        return Ok(Token::RootDeclaration { array: true });
    }

    let (name_bytes, array) = if rest.ends_with(b"[]") {
        (&rest[..rest.len() - 2], true)
    } else {
        (rest, false)
    };

    let name = std::str::from_utf8(name_bytes)
        .map(|s| s.to_string())
        .unwrap_or_default();

    if !is_valid_section_name(&name) {
        return Err(NormError::InvalidSectionName { line, name });
    }
    Ok(Token::SectionHeader { name, array })
}

pub(crate) fn is_valid_section_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    true
}

enum FieldEnd {
    Comma,
    RecordEnd,
}

fn parse_record(
    bytes: &[u8],
    mut pos: usize,
    mut line: usize,
) -> Result<(Vec<Cell>, usize, usize), NormError> {
    let mut cells = Vec::new();
    loop {
        let (cell, new_pos, new_line, end) = parse_field(bytes, pos, line)?;
        cells.push(cell);
        pos = new_pos;
        line = new_line;
        if matches!(end, FieldEnd::RecordEnd) {
            break;
        }
    }
    Ok((cells, pos, line))
}

fn parse_field(
    bytes: &[u8],
    pos: usize,
    mut line: usize,
) -> Result<(Cell, usize, usize, FieldEnd), NormError> {
    let mut i = pos;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }

    if i < bytes.len() && bytes[i] == b'"' {
        let mut content = String::new();
        i += 1;
        loop {
            if i >= bytes.len() {
                break;
            }
            let b = bytes[i];
            if b == 0 {
                return Err(NormError::NullByte { line });
            }
            if b == b'"' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                    content.push('"');
                    i += 2;
                    continue;
                }
                i += 1;
                break;
            }
            if b == b'\n' {
                content.push('\n');
                line += 1;
                i += 1;
                continue;
            }
            if b == b'\r' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    content.push('\n');
                    line += 1;
                    i += 2;
                    continue;
                }
                content.push('\r');
                i += 1;
                continue;
            }
            let clen = utf8_len(b);
            let end = (i + clen).min(bytes.len());
            if let Ok(s) = std::str::from_utf8(&bytes[i..end]) {
                content.push_str(s);
            }
            i = end;
        }
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        let (next_pos, new_line, end) = finish_field(bytes, i, line);
        return Ok((Cell::Quoted(content), next_pos, new_line, end));
    }

    let mut j = pos;
    while j < bytes.len() && bytes[j] != b',' && bytes[j] != b'\n' && bytes[j] != b'\r' {
        if bytes[j] == 0 {
            return Err(NormError::NullByte { line });
        }
        j += 1;
    }
    let raw = &bytes[pos..j];
    let trimmed = trim_ws(raw);
    let s = std::str::from_utf8(trimmed)
        .map(|s| s.to_string())
        .unwrap_or_default();
    let cell = Cell::Unquoted(s);
    let (next_pos, new_line, end) = finish_field(bytes, j, line);
    Ok((cell, next_pos, new_line, end))
}

fn finish_field(bytes: &[u8], i: usize, line: usize) -> (usize, usize, FieldEnd) {
    if i >= bytes.len() {
        return (i, line, FieldEnd::RecordEnd);
    }
    match bytes[i] {
        b',' => (i + 1, line, FieldEnd::Comma),
        b'\n' => (i + 1, line + 1, FieldEnd::RecordEnd),
        b'\r' => {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                (i + 2, line + 1, FieldEnd::RecordEnd)
            } else {
                (i + 1, line, FieldEnd::RecordEnd)
            }
        }
        _ => (i, line, FieldEnd::RecordEnd),
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_ok(s: &str) -> Vec<(usize, Token)> {
        lex(s).expect("lex should succeed")
    }

    #[test]
    fn rejects_bom() {
        let input = "\u{feff}:root\n";
        assert_eq!(lex(input), Err(NormError::BomDetected));
    }

    #[test]
    fn rejects_null_byte() {
        let input = ":root\n\0\n";
        assert_eq!(lex(input), Err(NormError::NullByte { line: 2 }));
    }

    #[test]
    fn classifies_root_declarations() {
        let toks = lex_ok(":root\n");
        assert_eq!(toks, vec![(1, Token::RootDeclaration { array: false })]);
        let toks = lex_ok(":root[]\n");
        assert_eq!(toks, vec![(1, Token::RootDeclaration { array: true })]);
    }

    #[test]
    fn classifies_section_headers() {
        let toks = lex_ok(":root\n:items\n:tags[]\n");
        assert_eq!(
            toks,
            vec![
                (1, Token::RootDeclaration { array: false }),
                (
                    2,
                    Token::SectionHeader {
                        name: "items".into(),
                        array: false,
                    }
                ),
                (
                    3,
                    Token::SectionHeader {
                        name: "tags".into(),
                        array: true,
                    }
                ),
            ]
        );
    }

    #[test]
    fn rejects_invalid_section_name() {
        let err = lex(":root\n:123bad\n").unwrap_err();
        assert!(matches!(err, NormError::InvalidSectionName { line: 2, .. }));
    }

    #[test]
    fn strips_inline_comments_on_structural_lines() {
        let toks = lex_ok(":root[]  # array root\n:data  # the data\n");
        assert_eq!(
            toks,
            vec![
                (1, Token::RootDeclaration { array: true }),
                (
                    2,
                    Token::SectionHeader {
                        name: "data".into(),
                        array: false,
                    }
                ),
            ]
        );
    }

    #[test]
    fn classifies_blank_and_comment() {
        let toks = lex_ok("# comment\n\n:root\n");
        assert_eq!(
            toks,
            vec![
                (1, Token::Comment),
                (2, Token::Blank),
                (3, Token::RootDeclaration { array: false }),
            ]
        );
    }

    #[test]
    fn parses_simple_data_row() {
        let toks = lex_ok(":root\n:data\na,b,c\n1,2,3\n");
        let row1 = &toks[2].1;
        let row2 = &toks[3].1;
        assert!(matches!(row1, Token::DataRow { .. }));
        assert!(matches!(row2, Token::DataRow { .. }));
        if let Token::DataRow { cells } = row1 {
            assert_eq!(
                cells,
                &vec![
                    Cell::Unquoted("a".into()),
                    Cell::Unquoted("b".into()),
                    Cell::Unquoted("c".into()),
                ]
            );
        }
    }

    #[test]
    fn distinguishes_empty_cell_from_empty_string() {
        let toks = lex_ok(":root\n:data\na,b\n\"\",\n");
        if let Token::DataRow { cells } = &toks[3].1 {
            assert_eq!(cells[0], Cell::Quoted("".into()));
            assert_eq!(cells[1], Cell::Unquoted("".into()));
        } else {
            panic!("expected data row");
        }
    }

    #[test]
    fn preserves_hash_in_data_row() {
        let toks = lex_ok(":root\n:data\ncolor\ncolor #FF0000\n");
        if let Token::DataRow { cells } = &toks[3].1 {
            assert_eq!(cells[0], Cell::Unquoted("color #FF0000".into()));
        } else {
            panic!("expected data row");
        }
    }

    #[test]
    fn handles_quoted_field_with_newline() {
        let toks = lex_ok(":root\n:data\na\n\"line one\nline two\"\n");
        if let Token::DataRow { cells } = &toks[3].1 {
            assert_eq!(cells[0], Cell::Quoted("line one\nline two".into()));
        } else {
            panic!("expected data row");
        }
    }

    #[test]
    fn handles_escaped_double_quote() {
        let toks = lex_ok(":root\n:data\na\n\"he said \"\"hi\"\"\"\n");
        if let Token::DataRow { cells } = &toks[3].1 {
            assert_eq!(cells[0], Cell::Quoted("he said \"hi\"".into()));
        } else {
            panic!("expected data row");
        }
    }

    #[test]
    fn trims_unquoted_whitespace() {
        let toks = lex_ok(":root\n:data\na\n  hello  \n");
        if let Token::DataRow { cells } = &toks[3].1 {
            assert_eq!(cells[0], Cell::Unquoted("hello".into()));
        } else {
            panic!("expected data row");
        }
    }

    #[test]
    fn preserves_quoted_whitespace() {
        let toks = lex_ok(":root\n:data\na\n\"  spaced  \"\n");
        if let Token::DataRow { cells } = &toks[3].1 {
            assert_eq!(cells[0], Cell::Quoted("  spaced  ".into()));
        } else {
            panic!("expected data row");
        }
    }

    #[test]
    fn strips_crlf() {
        let toks = lex_ok(":root\r\n:data\r\na\r\n1\r\n");
        assert_eq!(toks.len(), 4);
    }
}
