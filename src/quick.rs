//! Pure quick-add parser; positions are UTF-8 byte ranges in the original text.
use crate::{dates, model::Project};
use chrono::NaiveDate;
use std::ops::Range;
#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub title: String,
    pub date: Option<NaiveDate>,
    pub project_id: Option<u64>,
    pub new_project_name: Option<String>,
    pub priority: u8,
    pub tokens: Vec<Range<usize>>,
}
/// Splits `#tag` words off a task line; the rest keeps its words in order.
pub fn split_tags(line: &str) -> (String, Vec<String>) {
    let mut tags = Vec::new();
    let rest: Vec<&str> = line
        .split_whitespace()
        .filter(
            |word| match word.strip_prefix('#').filter(|t| !t.is_empty()) {
                Some(tag) => {
                    crate::model::push_tag(&mut tags, tag);
                    false
                }
                None => true,
            },
        )
        .collect();
    (rest.join(" "), tags)
}

/// Like `split_tags` for free text: line breaks and the spacing of the other words stay as they are.
pub fn split_text_tags(text: &str) -> (String, Vec<String>) {
    let mut tags = Vec::new();
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            line.split(' ')
                .filter(
                    |word| match word.strip_prefix('#').filter(|t| !t.is_empty()) {
                        Some(tag) if !tag.contains(char::is_whitespace) => {
                            crate::model::push_tag(&mut tags, tag);
                            false
                        }
                        _ => true,
                    },
                )
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    (lines.join("\n").trim_end().to_owned(), tags)
}

pub fn parse_quick_input(text: &str, now: NaiveDate, projects: &[Project]) -> Parsed {
    let mut result = Parsed {
        title: String::new(),
        date: None,
        project_id: None,
        new_project_name: None,
        priority: 0,
        tokens: vec![],
    };
    let words: Vec<_> = text.split_whitespace().collect();
    let mut offset = 0;
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        let start = offset + text[offset..].find(word).unwrap_or(0);
        let mut end = start + word.len();
        let mut consumed = 1;
        let mut recognized = false;
        if let Some(name) = word.strip_prefix('#').filter(|s| !s.is_empty()) {
            let key = name.to_lowercase();
            let normalize = |s: &str| s.to_lowercase().split_whitespace().collect::<String>();
            let exact = projects
                .iter()
                .find(|p| !p.archived && normalize(&p.name) == key);
            let matches: Vec<_> = projects
                .iter()
                .filter(|p| !p.archived && normalize(&p.name).starts_with(&key))
                .collect();
            if let Some(p) = exact.or_else(|| (matches.len() == 1).then(|| matches[0])) {
                result.project_id = Some(p.id);
                result.new_project_name = None;
            } else {
                result.new_project_name = Some(name.into());
            }
            recognized = true;
        } else if let Some(value) = word.strip_prefix('!') {
            let value = match value.to_lowercase().as_str() {
                "1" | "niedrig" => Some(1),
                "2" | "mittel" => Some(2),
                "3" | "hoch" => Some(3),
                _ => None,
            };
            if let Some(value) = value {
                result.priority = value;
                recognized = true;
            }
        } else if word.starts_with('@') {
            for count in (1..=3.min(words.len() - i)).rev() {
                let candidate = words[i..i + count].join(" ");
                if let Some(date) = dates::parse(&candidate[1..], now) {
                    result.date = Some(date);
                    consumed = count;
                    let last = words[i + count - 1];
                    end = start + text[start..].find(last).unwrap_or(0) + last.len();
                    recognized = true;
                    break;
                }
            }
        }
        if recognized {
            result.tokens.push(start..end);
        } else {
            if !result.title.is_empty() {
                result.title.push(' ');
            }
            result.title.push_str(
                word.strip_prefix('\\')
                    .filter(|_| {
                        word.starts_with("\\#")
                            || word.starts_with("\\@")
                            || word.starts_with("\\!")
                    })
                    .unwrap_or(word),
            );
        }
        offset = end;
        i += consumed;
    }
    result
}

/// Type tokens extend the existing quick-add syntax for tasks.
pub fn task_kind(text: &str) -> (crate::model::ChangeKind, String) {
    use crate::model::ChangeKind;
    let mut kind = ChangeKind::Added;
    let mut words = Vec::new();
    for word in text.split_whitespace() {
        match word.to_lowercase().as_str() {
            ":added" => kind = ChangeKind::Added,
            ":changed" => kind = ChangeKind::Changed,
            ":removed" => kind = ChangeKind::Removed,
            _ => words.push(word),
        }
    }
    (kind, words.join(" "))
}

#[cfg(test)]
mod tag_tests {
    use super::*;

    #[test]
    fn split_text_tags_keeps_lines_and_indentation() {
        let (text, tags) = split_text_tags("Milch #einkauf\n  - Brot  frisch\n#Einkauf #bio");
        assert_eq!(text, "Milch\n  - Brot  frisch");
        assert_eq!(tags, vec!["einkauf".to_owned(), "bio".to_owned()]);
        assert_eq!(split_text_tags("#solo").0, "");
    }

    #[test]
    fn split_tags_removes_tag_words_and_keeps_the_rest() {
        let (title, tags) = split_tags("Fix #Bug login #api #bug  !2");
        assert_eq!(title, "Fix login !2");
        assert_eq!(tags, vec!["Bug".to_owned(), "api".to_owned()]);
        assert_eq!(split_tags("# alone"), ("# alone".to_owned(), vec![]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn syntax_and_positions() {
        let now = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let p = parse_quick_input("Test @in 3 tagen !hoch #Neu", now, &[]);
        assert_eq!(p.title, "Test");
        assert_eq!(p.priority, 3);
        assert_eq!(p.date, NaiveDate::from_ymd_opt(2026, 11, 14));
        assert_eq!(p.new_project_name.as_deref(), Some("Neu"));
        assert_eq!(p.tokens[0], 5..16);
        assert_eq!(
            parse_quick_input(r"Text \#privat", now, &[]).title,
            "Text #privat"
        );
    }
}
