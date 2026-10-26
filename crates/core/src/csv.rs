//! Minimal RFC 4180 CSV writer, safe to open in Excel/LibreOffice.

/// Builds a CSV document: UTF-8 with BOM (so Excel on Windows detects the
/// encoding), CRLF line endings, quoted when needed. Cells that a spreadsheet
/// would treat as a formula (`= + - @`, tab, CR) are prefixed with `'` so a
/// tool name like `=HYPERLINK(...)` can't run as a formula.
pub fn write(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::from("\u{feff}");
    push_row(&mut out, headers.iter().map(|h| h.to_string()));
    for r in rows {
        push_row(&mut out, r.iter().cloned());
    }
    out
}

fn push_row(out: &mut String, cells: impl Iterator<Item = String>) {
    let mut first = true;
    for c in cells {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&cell(&c));
    }
    out.push_str("\r\n");
}

pub fn cell(raw: &str) -> String {
    let guarded = match raw.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') if !looks_numeric(raw) => format!("'{raw}"),
        _ => raw.to_string(),
    };
    if guarded.contains([',', '"', '\n', '\r'])
        || guarded.starts_with(' ')
        || guarded.ends_with(' ')
    {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

/// Negative numbers like "-12.50" are data, not formulas.
fn looks_numeric(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_escapes() {
        assert_eq!(cell("plain"), "plain");
        assert_eq!(cell("a,b"), "\"a,b\"");
        assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(cell("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn neutralizes_formulas_but_not_negative_numbers() {
        assert_eq!(cell("=1+1"), "'=1+1");
        assert_eq!(cell("@SUM(A1)"), "'@SUM(A1)");
        assert_eq!(cell("+cmd"), "'+cmd");
        assert_eq!(cell("-12.5"), "-12.5");
    }

    #[test]
    fn document_shape() {
        let doc = write(&["Name", "Count"], &[vec!["Sam".into(), "3".into()]]);
        assert!(doc.starts_with('\u{feff}'));
        assert_eq!(
            doc.trim_start_matches('\u{feff}'),
            "Name,Count\r\nSam,3\r\n"
        );
    }
}
