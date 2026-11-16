//! Markdown-Hilfen: Editor-Bearbeitungen (reine Funktionen auf Text + Auswahl),
//! Export und Changelog. Offsets sind immer UTF-8-Byteoffsets.

use crate::model::{ChangeKind, Task, Update};
use std::ops::Range;

/// Ersetze `range` durch `text` und setze danach die Auswahl auf `sel`
/// (Offsets beziehen sich auf den neuen Text).
#[derive(Debug, PartialEq, Eq)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
    pub sel: Range<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Heading,
    List,
    TaskList,
    Code,
    Link,
    Quote,
}

fn line_start(text: &str, pos: usize) -> usize {
    text[..pos].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(text: &str, pos: usize) -> usize {
    text[pos..].find('\n').map_or(text.len(), |i| pos + i)
}

struct ListPrefix {
    indent: String,
    marker: String,
    checkbox: bool,
    /// Länge des gesamten Präfixes inklusive Leerzeichen danach.
    len: usize,
}

fn parse_list_prefix(line: &str) -> Option<ListPrefix> {
    let trimmed = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - trimmed.len()];
    let bytes = trimmed.as_bytes();
    let marker_len = match bytes.first()? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if matches!(bytes.get(digits), Some(b'.') | Some(b')')) {
                digits + 1
            } else {
                return None;
            }
        }
        _ => return None,
    };
    if bytes.get(marker_len) != Some(&b' ') {
        return None;
    }
    let mut len = indent.len() + marker_len + 1;
    let rest = &trimmed[marker_len + 1..];
    let checkbox = rest.starts_with("[ ] ") || rest.starts_with("[x] ") || rest.starts_with("[X] ");
    if checkbox {
        len += 4;
    }
    Some(ListPrefix {
        indent: indent.to_owned(),
        marker: trimmed[..marker_len].to_owned(),
        checkbox,
        len,
    })
}

fn next_marker(marker: &str) -> String {
    let digits: String = marker.chars().take_while(char::is_ascii_digit).collect();
    match digits.parse::<u64>() {
        Ok(n) if !digits.is_empty() => format!("{}{}", n + 1, &marker[digits.len()..]),
        _ => marker.to_owned(),
    }
}

/// Enter in einer Liste: Liste fortsetzen; leerer Listenpunkt beendet sie.
pub fn list_enter(text: &str, sel: Range<usize>) -> Option<Edit> {
    if sel.start != sel.end || sel.end > text.len() {
        return None;
    }
    let cursor = sel.start;
    let start = line_start(text, cursor);
    let end = line_end(text, cursor);
    let line = &text[start..end];
    let prefix = parse_list_prefix(line)?;
    if cursor < start + prefix.len {
        return None;
    }
    if line[prefix.len..].trim().is_empty() {
        return Some(Edit {
            range: start..end,
            text: String::new(),
            sel: start..start,
        });
    }
    let mut insert = format!("\n{}{} ", prefix.indent, next_marker(&prefix.marker));
    if prefix.checkbox {
        insert.push_str("[ ] ");
    }
    let caret = cursor + insert.len();
    Some(Edit {
        range: cursor..cursor,
        text: insert,
        sel: caret..caret,
    })
}

/// Tab / Shift+Tab: Listenpunkte (oder markierte Zeilen) ein- bzw. ausrücken.
pub fn indent_lines(text: &str, sel: Range<usize>, outdent: bool) -> Option<Edit> {
    if sel.end > text.len() {
        return None;
    }
    let collapsed = sel.start == sel.end;
    if collapsed
        && !outdent
        && parse_list_prefix(&text[line_start(text, sel.start)..line_end(text, sel.start)])
            .is_none()
    {
        let caret = sel.start + 2;
        return Some(Edit {
            range: sel.start..sel.start,
            text: "  ".into(),
            sel: caret..caret,
        });
    }
    let last = if !collapsed && sel.end > 0 && text.as_bytes()[sel.end - 1] == b'\n' {
        sel.end - 1
    } else {
        sel.end
    };
    let start = line_start(text, sel.start);
    let end = line_end(text, last);
    let mut out = String::new();
    let mut first_delta: isize = 0;
    let mut total_delta: isize = 0;
    for (index, line) in text[start..end].split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let delta: isize = if outdent {
            let remove = if line.starts_with('\t') {
                1
            } else {
                line.bytes().take(2).take_while(|b| *b == b' ').count()
            };
            out.push_str(&line[remove..]);
            -(remove as isize)
        } else {
            out.push_str("  ");
            out.push_str(line);
            2
        };
        if index == 0 {
            first_delta = delta;
        }
        total_delta += delta;
    }
    if out == text[start..end] {
        return None;
    }
    let sel = if collapsed {
        let caret = (sel.start as isize + first_delta).max(start as isize) as usize;
        caret..caret
    } else {
        start..(end as isize + total_delta) as usize
    };
    Some(Edit {
        range: start..end,
        text: out,
        sel,
    })
}

/// Automatisches Schließen von Klammern/Backticks beim Tippen.
pub fn auto_pair(text: &str, sel: Range<usize>, typed: char) -> Option<Edit> {
    if sel.end > text.len() {
        return None;
    }
    let closer = match typed {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '`' => '`',
        ')' | ']' | '}' => typed,
        _ => return None,
    };
    let collapsed = sel.start == sel.end;
    let next = text[sel.end..].chars().next();
    let prev_two = &text[..sel.start];
    if collapsed && matches!(typed, ')' | ']' | '}') {
        return (next == Some(typed)).then(|| Edit {
            range: sel.start..sel.start,
            text: String::new(),
            sel: sel.start + 1..sel.start + 1,
        });
    }
    if matches!(typed, ')' | ']' | '}') {
        return None;
    }
    if collapsed && typed == '`' {
        if next == Some('`') {
            return Some(Edit {
                range: sel.start..sel.start,
                text: String::new(),
                sel: sel.start + 1..sel.start + 1,
            });
        }
        if prev_two.ends_with("``") {
            return None;
        }
    }
    if collapsed && next.is_some_and(|c| !c.is_whitespace() && !")]}`".contains(c)) {
        return None;
    }
    let inner = &text[sel.clone()];
    let replacement = format!("{typed}{inner}{closer}");
    let new_sel = if collapsed {
        sel.start + 1..sel.start + 1
    } else {
        sel.start + 1..sel.start + 1 + inner.len()
    };
    Some(Edit {
        range: sel,
        text: replacement,
        sel: new_sel,
    })
}

/// Einfügen einer URL über markierten Text erzeugt einen Link.
pub fn paste_link(text: &str, sel: Range<usize>, clipboard: &str) -> Option<Edit> {
    let url = clipboard.trim();
    let is_url = (url.starts_with("http://") || url.starts_with("https://"))
        && !url.chars().any(char::is_whitespace);
    if sel.start == sel.end || sel.end > text.len() || !is_url {
        return None;
    }
    let label = &text[sel.clone()];
    let replacement = format!("[{label}]({url})");
    let caret = sel.start + replacement.len();
    Some(Edit {
        range: sel,
        text: replacement,
        sel: caret..caret,
    })
}

fn inline_wrap(text: &str, sel: Range<usize>, marker: &str) -> Edit {
    let (start, end) = (sel.start, sel.end);
    let before = &text[..start];
    let after = &text[end..];
    if start != end && before.ends_with(marker) && after.starts_with(marker) {
        // Entfernen
        let inner = text[start..end].to_owned();
        let from = start - marker.len();
        return Edit {
            range: from..end + marker.len(),
            sel: from..from + inner.len(),
            text: inner,
        };
    }
    let inner = &text[start..end];
    let replacement = format!("{marker}{inner}{marker}");
    let new_sel = start + marker.len()..start + marker.len() + inner.len();
    Edit {
        range: start..end,
        text: replacement,
        sel: new_sel,
    }
}

fn line_prefix(text: &str, sel: Range<usize>, prefix: &str) -> Edit {
    let last = if sel.end > sel.start && text.as_bytes()[sel.end - 1] == b'\n' {
        sel.end - 1
    } else {
        sel.end
    };
    let start = line_start(text, sel.start);
    let end = line_end(text, last);
    let lines: Vec<&str> = text[start..end].split('\n').collect();
    let all_prefixed = lines.iter().all(|line| line.starts_with(prefix));
    let mut out = String::new();
    let mut first_delta: isize = 0;
    let mut total: isize = 0;
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let delta = if all_prefixed {
            out.push_str(&line[prefix.len()..]);
            -(prefix.len() as isize)
        } else if line.starts_with(prefix) {
            out.push_str(line);
            0
        } else {
            out.push_str(prefix);
            out.push_str(line);
            prefix.len() as isize
        };
        if index == 0 {
            first_delta = delta;
        }
        total += delta;
    }
    let new_sel = if sel.start == sel.end {
        let caret = (sel.start as isize + first_delta).max(start as isize) as usize;
        caret..caret
    } else {
        start..(end as isize + total) as usize
    };
    Edit {
        range: start..end,
        text: out,
        sel: new_sel,
    }
}

/// Werkzeugleiste: wendet ein Format auf die Auswahl an.
pub fn apply_format(text: &str, sel: Range<usize>, format: Format) -> Edit {
    let sel = sel.start.min(text.len())..sel.end.min(text.len());
    match format {
        Format::Bold => inline_wrap(text, sel, "**"),
        Format::Italic => inline_wrap(text, sel, "*"),
        Format::Code => inline_wrap(text, sel, "`"),
        Format::Heading => line_prefix(text, sel, "## "),
        Format::List => line_prefix(text, sel, "- "),
        Format::TaskList => line_prefix(text, sel, "- [ ] "),
        Format::Quote => line_prefix(text, sel, "> "),
        Format::Link => {
            let inner = if sel.start == sel.end {
                "Text"
            } else {
                &text[sel.clone()]
            };
            let replacement = format!("[{inner}](https://)");
            // Bei Auswahl: URL markieren, sonst den Platzhalter "Text".
            let new_sel = if sel.start == sel.end {
                sel.start + 1..sel.start + 1 + inner.len()
            } else {
                let url_start = sel.start + inner.len() + 3;
                url_start..url_start + "https://".len()
            };
            Edit {
                range: sel,
                text: replacement,
                sel: new_sel,
            }
        }
    }
}

/// Schreibt den neuen Zustand einer Aufgabenlisten-Checkbox (`marker` = `[ ]` / `[x]`).
pub fn set_task_marker(text: &str, marker: Range<usize>, checked: bool) -> Option<String> {
    let slot = text.get(marker.clone())?;
    if slot.len() != 3 || !slot.starts_with('[') || !slot.ends_with(']') {
        return None;
    }
    let mut out = text.to_owned();
    out.replace_range(marker, if checked { "[x]" } else { "[ ]" });
    Some(out)
}

pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn yaml_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', " ")
    )
}

/// Export eines Updates als Markdown mit Frontmatter und Checkliste.
pub fn export_update(update: &Update) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("title: {}\n", yaml_quote(&update.title)));
    out.push_str(&format!("status: {}\n", update.status.label()));
    if let Some(date) = update.planned_for {
        out.push_str(&format!("date: {}\n", date.format("%Y-%m-%d")));
    }
    out.push_str("---\n\n");
    let notes = update.notes.trim_end();
    if !notes.is_empty() {
        out.push_str(notes);
        out.push_str("\n\n");
    }
    if !update.tasks.is_empty() {
        out.push_str("## Aufgaben\n\n");
        for task in &update.tasks {
            out.push_str(if task.sub { "  - [" } else { "- [" });
            out.push(if task.done { 'x' } else { ' ' });
            out.push_str("] ");
            out.push_str(&task.title);
            if let Some(date) = task.planned_for {
                out.push_str(&format!(" ({})", date.format("%Y-%m-%d")));
            }
            out.push('\n');
            let description = task.description.trim();
            if !description.is_empty() {
                let pad = if task.sub { "      " } else { "    " };
                for line in description.lines() {
                    out.push_str(&format!("{pad}{line}\n"));
                }
            }
        }
    }
    out
}

/// Einzige Stelle, die das Standard-Changelog-Format erzeugt: Abschnitte Added/Changed/Removed,
/// Einträge `* HEADING: Beschreibung` (ohne Beschreibung nur `* HEADING`), leere Abschnitte entfallen.
/// Ohne Datum, Emojis oder Links. Mit `version` beginnt der Text mit `## <version>`.
pub fn format_changelog(version: Option<&str>, tasks: &[Task], only_done: bool) -> String {
    let mut sections = Vec::new();
    for kind in ChangeKind::ALL {
        let mut lines = Vec::new();
        let mut index = 0;
        while index < tasks.len() {
            let task = &tasks[index];
            index += 1;
            let subs: Vec<&str> = tasks[index..]
                .iter()
                .take_while(|t| t.sub)
                .map(|t| t.title.trim())
                .collect();
            if task.sub || task.kind != kind || (only_done && !task.done) {
                continue;
            }
            let mut description = task.description.trim().replace('\n', " ");
            if description.is_empty() && !subs.is_empty() {
                description = subs.join("; ");
            }
            let title = task.title.trim().replace('\n', " ");
            lines.push(if description.is_empty() {
                format!("* {title}")
            } else {
                format!("* {title}: {description}")
            });
        }
        if !lines.is_empty() {
            sections.push(format!(
                "### {}
{}",
                kind.label(),
                lines.join(
                    "
"
                )
            ));
        }
    }
    let body = sections.join(
        "

",
    );
    match version {
        Some(version) => release_block(version, &body),
        None => body,
    }
}

/// Ein Release im Standardformat: `## <version>`, Leerzeile, Changelog-Text.
pub fn release_block(version: &str, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        format!("## {}", version.trim())
    } else {
        format!(
            "## {}

{body}",
            version.trim()
        )
    }
}

/// Datei mit allen Versionen, neueste zuerst, Blöcke durch eine Leerzeile getrennt.
pub fn changelog_file(releases: &[(&str, &str)]) -> String {
    let blocks: Vec<String> = releases
        .iter()
        .map(|(version, body)| release_block(version, body))
        .collect();
    format!(
        "{}
",
        blocks.join(
            "

"
        )
    )
}

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

pub fn is_version(text: &str) -> bool {
    parse_version(text).is_some()
}

/// Ist `candidate` eine gültige und höhere Version als `current`?
pub fn is_newer(candidate: &str, current: &str) -> bool {
    matches!((parse_version(candidate), parse_version(current)), (Some(a), Some(b)) if a > b)
}

/// Nächste Version nach der höchsten gültigen `x.y.z` der Liste (Minor-Sprung oder Patch).
pub fn suggest_version(existing: &[&str], minor: bool) -> Option<String> {
    let (major, mi, patch) = existing.iter().filter_map(|v| parse_version(v)).max()?;
    Some(if minor {
        format!("{major}.{}.0", mi + 1)
    } else {
        format!("{major}.{mi}.{}", patch + 1)
    })
}

/// Dateiname-tauglicher Titel.
pub fn file_stem(title: &str) -> String {
    let stem: String = title
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "update".into()
    } else {
        stem.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Status, Task};

    fn apply(text: &str, edit: &Edit) -> String {
        let mut out = text.to_owned();
        out.replace_range(edit.range.clone(), &edit.text);
        out
    }

    #[test]
    fn enter_continues_and_ends_lists() {
        let text = "- eins";
        let edit = list_enter(text, 6..6).unwrap();
        assert_eq!(apply(text, &edit), "- eins\n- ");
        assert_eq!(edit.sel, 9..9);
        let text = "1. a\n2. ";
        let edit = list_enter(text, text.len()..text.len()).unwrap();
        assert_eq!(apply(text, &edit), "1. a\n");
        let text = "  3. x";
        assert_eq!(
            apply(text, &list_enter(text, 6..6).unwrap()),
            "  3. x\n  4. "
        );
        let text = "- [x] fertig";
        assert_eq!(
            apply(text, &list_enter(text, text.len()..text.len()).unwrap()),
            "- [x] fertig\n- [ ] "
        );
        assert!(list_enter("kein Listenpunkt", 3..3).is_none());
    }

    #[test]
    fn tab_indents_and_outdents() {
        let text = "- a\n- b";
        let edit = indent_lines(text, 7..7, false).unwrap();
        assert_eq!(apply(text, &edit), "- a\n  - b");
        assert_eq!(edit.sel, 9..9);
        let text = "- a\n  - b";
        assert_eq!(
            apply(text, &indent_lines(text, 9..9, true).unwrap()),
            "- a\n- b"
        );
        assert!(indent_lines("- a", 3..3, true).is_none());
        // Normale Zeile: Tab fügt Leerzeichen ein.
        assert_eq!(
            apply("abc", &indent_lines("abc", 1..1, false).unwrap()),
            "a  bc"
        );
    }

    #[test]
    fn brackets_and_backticks_autoclose() {
        let edit = auto_pair("ab ", 3..3, '(').unwrap();
        assert_eq!(apply("ab ", &edit), "ab ()");
        assert_eq!(edit.sel, 4..4);
        // Überspringen des schließenden Zeichens
        let edit = auto_pair("ab ()", 4..4, ')').unwrap();
        assert_eq!(apply("ab ()", &edit), "ab ()");
        assert_eq!(edit.sel, 5..5);
        // Markierter Text wird umschlossen
        let edit = auto_pair("wort", 0..4, '`').unwrap();
        assert_eq!(apply("wort", &edit), "`wort`");
        assert_eq!(edit.sel, 1..5);
        // Kein Auto-Close vor Text
        assert!(auto_pair("abc", 0..0, '(').is_none());
        // Dreifach-Backtick bleibt möglich
        assert!(auto_pair("``", 2..2, '`').is_none());
    }

    #[test]
    fn format_toggles() {
        let edit = apply_format("text", 0..4, Format::Bold);
        assert_eq!(apply("text", &edit), "**text**");
        let again = apply_format("**text**", edit.sel.clone(), Format::Bold);
        assert_eq!(apply("**text**", &again), "text");
        let edit = apply_format("a\nb", 0..3, Format::List);
        assert_eq!(apply("a\nb", &edit), "- a\n- b");
        let edit = apply_format("- a\n- b", 0..7, Format::List);
        assert_eq!(apply("- a\n- b", &edit), "a\nb");
        let edit = apply_format("", 0..0, Format::Link);
        assert_eq!(apply("", &edit), "[Text](https://)");
    }

    #[test]
    fn paste_url_over_selection_makes_link() {
        let edit = paste_link("siehe docs", 6..10, " https://example.org ").unwrap();
        assert_eq!(
            apply("siehe docs", &edit),
            "siehe [docs](https://example.org)"
        );
        assert!(paste_link("x", 0..0, "https://a.b").is_none());
        assert!(paste_link("x", 0..1, "kein link").is_none());
    }

    #[test]
    fn task_marker_roundtrip() {
        let text = "- [ ] a\n- [x] b";
        let out = set_task_marker(text, 2..5, true).unwrap();
        assert_eq!(out, "- [x] a\n- [x] b");
        assert!(set_task_marker(text, 0..2, true).is_none());
    }

    fn sample() -> Update {
        let task = |id, title: &str, kind, sub| Task {
            priority: 0,
            id,
            title: title.into(),
            done: id == 1,
            planned_for: None,
            sub,
            kind,
            description: String::new(),
            remind: Default::default(),
        };
        Update {
            version: String::new(),
            created_at: 0,
            activity: Vec::new(),
            priority: 0,
            project_id: None,
            id: 1,
            title: "0.3.0".into(),
            notes: "# Das hier\n\n```\ncargo run --locked\n```".into(),
            status: Status::InArbeit,
            planned_for: chrono::NaiveDate::from_ymd_opt(2026, 11, 14),
            tasks: vec![
                task(1, "Markdown", ChangeKind::Added, false),
                task(2, "Vorschau", ChangeKind::Added, true),
                task(3, "Statusleiste", ChangeKind::Removed, false),
            ],
            archived: false,
            deleted_on: None,
            edited_at: 0,
            release: None,
            remind: Default::default(),
        }
    }

    #[test]
    fn export_contains_frontmatter_and_checklist() {
        let out = export_update(&sample());
        assert!(out.starts_with("---\ntitle: \"0.3.0\"\nstatus: In Arbeit\ndate: 2026-11-14\n---"));
        assert!(out.contains("- [x] Markdown\n  - [ ] Vorschau\n- [ ] Statusleiste"));
        assert!(out.contains("cargo run --locked"));
    }

    #[test]
    fn changelog_standard_format() {
        let mut update = sample();
        let mut plain = update.tasks[0].clone();
        plain.title = "Leer".into();
        plain.kind = ChangeKind::Changed;
        update.tasks.push(plain);
        let out = format_changelog(Some("1.2.0"), &update.tasks, false);
        assert!(out.starts_with(
            "## 1.2.0

### Added
* "
        ));
        assert!(out.contains(
            "

### Changed
* Leer
"
        ));
        assert!(
            !out.contains(
                "### Removed

"
            ) && !out.contains("202")
        );
        assert_eq!(format_changelog(None, &[], false), "");
        assert_eq!(
            changelog_file(&[
                (
                    "1.1.0",
                    "### Added
* A"
                ),
                (
                    "1.0.0",
                    "### Added
* B"
                )
            ]),
            "## 1.1.0

### Added
* A

## 1.0.0

### Added
* B
"
        );
    }

    #[test]
    fn version_suggestion() {
        assert_eq!(
            suggest_version(&["1.0.0", "1.2.3", "x", "1.2"], false).as_deref(),
            Some("1.2.4")
        );
        assert_eq!(suggest_version(&["0.9.2"], true).as_deref(), Some("0.10.0"));
        assert_eq!(suggest_version(&[], false), None);
        assert_eq!(suggest_version(&["abc", ""], true), None);
    }
}
