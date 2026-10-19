//! The terminal view behind `scanscan tui`.
//!
//! It renders the children of the current scope as sized bars and navigates by
//! line commands rather than raw keystrokes, so it works over SSH, inside pipes
//! and on terminals where no termios layer is available:
//!
//! ```text
//! <enter> | j   page down        k        page up
//! u | ..        go to the parent g        go to the snapshot root
//! o | <n>       open child n     s        cycle the sort order
//! m             cycle the metric + | -    wider / narrower bars
//! q             quit
//! ```

/// One rendered line: a label, the value the bar is scaled from, and a short
/// kind note.
#[derive(Debug, Clone)]
pub struct Row {
    pub label: String,
    pub value: u64,
    pub detail: String,
}

/// A parsed line command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Quit,
    Parent,
    Root,
    Open,
    Select(usize),
    PageDown,
    PageUp,
    Sort,
    Metric,
    Widen,
    Narrow,
    Unnown,
}

/// A bar of exactly `width` cells, filled in proportion to `max`.
pub fn bar(value: u64, max: u64, width: usize) -> String {
    let mut out = String::new();
    let filled = if max == 0 || width == 0 {
        0
    } else {
        ((value as f64 / max as f64) * width as f64) as usize
    };
    for i in 0..width {
        if i < filled {
            out.push_str("█");
        } else {
            out.push_str("░");
        }
    }
    out
}

/// Human-readable size in binary units.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Replace the control characters a terminal would execute (CSI/OSC sequences,
/// cursor moves, title writes) so an untrusted filename can only print, never
/// drive the terminal.
pub fn visible(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        let code = c as u32;
        if code < 0x20 || code == 0x7f || (code >= 0x80 && code <= 0x9f) {
            out.push('?');
        } else {
            out.push(c);
        }
    }
    out
}

/// One full frame: a header line, a breadcrumb, one line per row, a footer.
pub fn frame(
    header: &str,
    crumb: &str,
    rows: &Vec<Row>,
    max: u64,
    width: usize,
    selected: usize,
    footer: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&visible(header));
    out.push_str("  ");
    out.push_str(&visible(crumb));
    out.push_str("\n");
    for i in 0..rows.len() {
        if let Some(row) = rows.get(i) {
            if i == selected {
                out.push_str("› ");
            } else {
                out.push_str("  ");
            }
            let size = format!("{:>10}  ", human_size(row.value));
            out.push_str(size.as_str());
            out.push_str(&bar(row.value, max, width));
            out.push_str("  ");
            out.push_str(&visible(row.label.as_str()));
            if !row.detail.is_empty() {
                out.push_str("  (");
                out.push_str(&visible(row.detail.as_str()));
                out.push_str(")");
            }
            out.push_str("\n");
        }
    }
    out.push_str(&visible(footer));
    out.push_str("\n");
    out
}

/// Parse one line command. An empty line pages down, a bare number selects a
/// row by its 1-based position.
pub fn parse_nav(line: &str) -> Nav {
    let trimmed = line.trim();
    match trimmed {
        "q" | "quit" => Nav::Quit,
        "u" | ".." | "up" => Nav::Parent,
        "g" | "top" | "root" => Nav::Root,
        "o" | "open" => Nav::Open,
        "" | "j" | "down" => Nav::PageDown,
        "k" | "up-page" => Nav::PageUp,
        "s" | "sort" => Nav::Sort,
        "m" | "metric" => Nav::Metric,
        "+" | "widen" => Nav::Widen,
        "-" | "narrow" => Nav::Narrow,
        other => match parse_index(other) {
            Some(n) => Nav::Select(n),
            None => Nav::Unnown,
        },
    }
}

fn parse_index(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let mut value = 0usize;
    for c in s.chars() {
        if c < '0' || c > '9' {
            return None;
        }
        value = value * 10 + (c as usize - '0' as usize);
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_fills_in_proportion_to_the_maximum() {
        assert_eq!(bar(0, 0, 4).as_str(), "░░░░");
        assert_eq!(bar(10, 10, 4).as_str(), "████");
        assert_eq!(bar(5, 10, 4).as_str(), "██░░");
        assert_eq!(bar(1, 1000, 2).as_str(), "░░");
        assert!(bar(7, 10, 0).is_empty());
    }

    #[test]
    fn human_size_reads_binary_units() {
        assert_eq!(human_size(0).as_str(), "0.0 B");
        assert_eq!(human_size(2048).as_str(), "2.0 KB");
        assert_eq!(human_size(1_073_741_824).as_str(), "1.0 GB");
    }

    #[test]
    fn parse_nav_reads_line_commands() {
        assert!(matches!(parse_nav("q"), Nav::Quit));
        assert!(matches!(parse_nav(""), Nav::PageDown));
        assert!(matches!(parse_nav("j"), Nav::PageDown));
        assert!(matches!(parse_nav("u"), Nav::Parent));
        assert!(matches!(parse_nav(".."), Nav::Parent));
        assert!(matches!(parse_nav("7"), Nav::Select(7)));
        assert!(matches!(parse_nav("zap"), Nav::Unnown));
    }

    #[test]
    fn visible_replaces_control_chars() {
        assert_eq!(visible("plain").as_str(), "plain");
        assert_eq!(visible("\x1b[2J").as_str(), "?[2J");
        assert_eq!(visible("a\x07b").as_str(), "a?b");
        assert_eq!(visible("dir/na\tme").as_str(), "dir/na?me");
    }

    #[test]
    fn frame_has_one_line_per_row_plus_header_and_footer() {
        let empty: Vec<Row> = Vec::new();
        assert_eq!(frame("h", "/", &empty, 0, 4, 0, "f").as_str(), "h  /\nf\n");
    }

    #[test]
    fn frame_marks_the_selected_row() {
        let rows = vec![
            Row {
                label: "a".into(),
                value: 10,
                detail: "file".into(),
            },
            Row {
                label: "b".into(),
                value: 5,
                detail: "dir".into(),
            },
        ];
        let selected = frame("h", "/", &rows, 10, 4, 1, "f");
        let beyond = frame("h", "/", &rows, 10, 4, 9, "f");
        assert!(selected != beyond);
    }
}
