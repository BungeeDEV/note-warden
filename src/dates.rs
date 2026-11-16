//! Date logic: German names, relative labels and natural-language input.
//! Date values are always local calendar days (`NaiveDate`), never UTC.

use crate::model::DateFormat;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

const MONTHS_SHORT: [&str; 12] = [
    "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez.",
];

pub fn weekday_short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Mo.",
        Weekday::Tue => "Di.",
        Weekday::Wed => "Mi.",
        Weekday::Thu => "Do.",
        Weekday::Fri => "Fr.",
        Weekday::Sat => "Sa.",
        Weekday::Sun => "So.",
    }
}

pub fn weekday_long(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Montag",
        Weekday::Tue => "Dienstag",
        Weekday::Wed => "Mittwoch",
        Weekday::Thu => "Donnerstag",
        Weekday::Fri => "Freitag",
        Weekday::Sat => "Samstag",
        Weekday::Sun => "Sonntag",
    }
}

/// "14. November 2026" bzw. "2026-11-14".
pub fn full(date: NaiveDate, format: DateFormat) -> String {
    match format {
        DateFormat::German => format!(
            "{}. {} {}",
            date.day(),
            MONTHS[date.month0() as usize],
            date.year()
        ),
        DateFormat::Iso => date.format("%Y-%m-%d").to_string(),
        DateFormat::Medium => format!(
            "{}. {} {}",
            date.day(),
            MONTHS_SHORT[date.month0() as usize],
            date.year()
        ),
        DateFormat::Numeric => date.format("%d.%m.%Y").to_string(),
    }
}

/// Month abbreviation for cells and chips ("Dez.").
pub fn month_short(date: NaiveDate) -> &'static str {
    MONTHS_SHORT[date.month0() as usize]
}

/// "Freitag, 14. November 2026" for tooltips.
pub fn full_with_weekday(date: NaiveDate, format: DateFormat) -> String {
    format!("{}, {}", weekday_long(date.weekday()), full(date, format))
}

/// Short form for chips: "Fr., 14. Nov." (in the same year) or "14. Nov. 2026".
fn short(date: NaiveDate, today: NaiveDate, format: DateFormat) -> String {
    match format {
        DateFormat::Iso => return date.format("%Y-%m-%d").to_string(),
        DateFormat::Numeric => return date.format("%d.%m.%Y").to_string(),
        _ => {}
    }
    let month = MONTHS_SHORT[date.month0() as usize];
    if date.year() == today.year() {
        format!(
            "{}, {}. {}",
            weekday_short(date.weekday()),
            date.day(),
            month
        )
    } else {
        format!("{}. {} {}", date.day(), month, date.year())
    }
}

/// Date chip without relative wording ("Fr., 14. Nov." or the date with the year).
pub fn absolute(date: NaiveDate, today: NaiveDate, format: DateFormat) -> String {
    short(date, today, format)
}

/// Relative text for date chips.
pub fn relative(date: NaiveDate, today: NaiveDate, format: DateFormat) -> String {
    let diff = (date - today).num_days();
    match diff {
        0 => "Heute".into(),
        1 => "Morgen".into(),
        -1 => "Gestern".into(),
        2..=6 => format!("In {diff} Tagen"),
        -6..=-2 => format!("Vor {} Tagen", -diff),
        _ => short(date, today, format),
    }
}

/// Monday of the week that contains `date`.
pub fn next_monday(today: NaiveDate) -> NaiveDate {
    today + Duration::days(7 - today.weekday().num_days_from_monday() as i64)
}

fn weekday_from_word(word: &str) -> Option<Weekday> {
    Some(match word.trim_end_matches('.') {
        "mo" | "mon" | "montag" => Weekday::Mon,
        "di" | "die" | "dienstag" => Weekday::Tue,
        "mi" | "mit" | "mittwoch" => Weekday::Wed,
        "do" | "don" | "donnerstag" => Weekday::Thu,
        "fr" | "fre" | "freitag" => Weekday::Fri,
        "sa" | "sam" | "samstag" => Weekday::Sat,
        "so" | "son" | "sonntag" => Weekday::Sun,
        _ => return None,
    })
}

/// Next occurrence of the weekday, always strictly after today.
fn next_weekday(today: NaiveDate, day: Weekday) -> NaiveDate {
    let mut diff =
        day.num_days_from_monday() as i64 - today.weekday().num_days_from_monday() as i64;
    if diff <= 0 {
        diff += 7;
    }
    today + Duration::days(diff)
}

fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let first = crate::model::shift_month(date, months);
    let last = crate::model::shift_month(first, 1) - Duration::days(1);
    first.with_day(date.day().min(last.day())).unwrap_or(first)
}

/// Versteht "morgen", "fr", "14.11.", "14.11.2026", "2026-11-14", "in 2 wochen" ...
pub fn parse(input: &str, today: NaiveDate) -> Option<NaiveDate> {
    let text = input.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }
    match text.as_str() {
        "heute" | "jetzt" => return Some(today),
        "morgen" => return Some(today + Duration::days(1)),
        "übermorgen" | "uebermorgen" => return Some(today + Duration::days(2)),
        "gestern" => return Some(today - Duration::days(1)),
        "nächste woche" | "naechste woche" | "nächsten montag" | "naechsten montag" => {
            return Some(next_monday(today));
        }
        _ => {}
    }
    if let Some(day) = weekday_from_word(&text) {
        return Some(next_weekday(today, day));
    }
    if let Some(rest) = text
        .strip_prefix("nächsten ")
        .or_else(|| text.strip_prefix("naechsten "))
        .or_else(|| text.strip_prefix("nächster "))
        .or_else(|| text.strip_prefix("am "))
        && let Some(day) = weekday_from_word(rest.trim())
    {
        return Some(next_weekday(today, day));
    }
    if let Some(rest) = text.strip_prefix("in ") {
        let mut parts = rest.split_whitespace();
        let count: i64 = parts.next()?.parse().ok()?;
        let unit = parts.next()?;
        if parts.next().is_some() || !(0..=3650).contains(&count) {
            return None;
        }
        return match unit {
            "tag" | "tagen" | "tage" | "t" | "d" => Some(today + Duration::days(count)),
            "woche" | "wochen" | "w" => Some(today + Duration::days(count * 7)),
            "monat" | "monaten" | "monate" | "m" => Some(add_months(today, count as i32)),
            _ => None,
        };
    }
    if let Ok(date) = NaiveDate::parse_from_str(&text, "%Y-%m-%d") {
        return Some(date);
    }
    // 14.11. / 14.11 / 14.11.26 / 14.11.2026
    let parts: Vec<&str> = text.trim_end_matches('.').split('.').collect();
    if (2..=3).contains(&parts.len()) {
        let day: u32 = parts[0].trim().parse().ok()?;
        let month: u32 = parts[1].trim().parse().ok()?;
        return match parts.get(2) {
            Some(year) => {
                let mut year: i32 = year.trim().parse().ok()?;
                if year < 100 {
                    year += 2000;
                }
                NaiveDate::from_ymd_opt(year, month, day)
            }
            None => {
                // Without a year: the next occurrence that is not in the past.
                let this = NaiveDate::from_ymd_opt(today.year(), month, day)?;
                if this >= today {
                    Some(this)
                } else {
                    NaiveDate::from_ymd_opt(today.year() + 1, month, day)
                }
            }
        };
    }
    None
}

/// Always compute calendar weeks from a local calendar day.
pub fn week_start(date: NaiveDate, start: crate::model::WeekStart) -> NaiveDate {
    date - Duration::days(start.offset(date.weekday()))
}
pub fn elapsed(at: i64) -> String {
    elapsed_opt(at).unwrap_or_default()
}
/// Full date with time for tooltips; `None` if the time is missing.
pub fn full_time(at: i64) -> Option<String> {
    use chrono::{Local, TimeZone};
    if at <= 0 {
        return None;
    }
    Local
        .timestamp_millis_opt(at)
        .single()
        .map(|d| d.format("%d.%m.%Y, %H:%M").to_string())
}
/// Relative time or `None` if the time is missing.
pub fn elapsed_opt(at: i64) -> Option<String> {
    (at > 0).then(|| elapsed_at(at, crate::model::now_millis()))
}
pub fn elapsed_at(at: i64, now: i64) -> String {
    use chrono::{Local, TimeZone};
    if at <= 0 {
        return String::new();
    }
    let minutes = ((now - at) / 60_000).max(0);
    if minutes == 0 {
        return "gerade eben".into();
    }
    if minutes < 60 {
        return format!("vor {minutes} Min.");
    }
    let Some(date) = Local.timestamp_millis_opt(at).single() else {
        return String::new();
    };
    let Some(current) = Local.timestamp_millis_opt(now).single() else {
        return String::new();
    };
    let days = (current.date_naive() - date.date_naive()).num_days();
    if days == 1 {
        return "gestern".into();
    }
    if minutes < 1440 {
        return format!("vor {} Std.", minutes / 60);
    }
    if days < 7 {
        return format!("vor {days} Tagen");
    }
    format!("{}. {}", date.day(), month_short(date.date_naive()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn relative_labels() {
        let today = d(2026, 11, 10); // Dienstag
        let f = DateFormat::German;
        assert_eq!(relative(today, today, f), "Heute");
        assert_eq!(relative(d(2026, 11, 11), today, f), "Morgen");
        assert_eq!(relative(d(2026, 11, 13), today, f), "In 3 Tagen");
        assert_eq!(relative(d(2026, 11, 20), today, f), "Fr., 20. Nov.");
        assert_eq!(relative(d(2027, 1, 5), today, f), "5. Jan. 2027");
    }

    #[test]
    fn natural_input() {
        let today = d(2026, 11, 10); // Dienstag
        assert_eq!(parse("morgen", today), Some(d(2026, 11, 11)));
        assert_eq!(parse("fr", today), Some(d(2026, 11, 13)));
        assert_eq!(parse("di", today), Some(d(2026, 11, 17)));
        assert_eq!(parse("14.11.", today), Some(d(2026, 11, 14)));
        assert_eq!(parse("1.1.", today), Some(d(2027, 1, 1)));
        assert_eq!(parse("14.11.2026", today), Some(d(2026, 11, 14)));
        assert_eq!(parse("in 2 wochen", today), Some(d(2026, 11, 24)));
        assert_eq!(parse("in 1 monat", today), Some(d(2026, 12, 10)));
        assert_eq!(parse("2026-12-24", today), Some(d(2026, 12, 24)));
        assert_eq!(parse("blabla", today), None);
        assert_eq!(parse("31.2.", today), None);
    }

    #[test]
    fn week_start_edge_cases() {
        use crate::model::WeekStart::{Monday, Sunday};
        // Do., 12. Nov. 2026 -> Montag 9.11.
        assert_eq!(week_start(d(2026, 11, 12), Monday), d(2026, 11, 9));
        // Wednesday 11.11. (earlier failure case): the week starts on 9.11., not on 16.11.
        assert_eq!(week_start(d(2026, 11, 11), Monday), d(2026, 11, 9));
        // Monday is its own week start, Sunday belongs to the week before.
        assert_eq!(week_start(d(2026, 11, 9), Monday), d(2026, 11, 9));
        assert_eq!(week_start(d(2026, 11, 15), Monday), d(2026, 11, 9));
        // Month and year boundaries.
        assert_eq!(week_start(d(2026, 12, 1), Monday), d(2026, 11, 30));
        assert_eq!(week_start(d(2027, 1, 1), Monday), d(2026, 12, 28));
        // Wochenstart Sonntag.
        assert_eq!(week_start(d(2026, 11, 15), Sunday), d(2026, 11, 15));
        assert_eq!(week_start(d(2026, 11, 14), Sunday), d(2026, 11, 8));
        assert_eq!(week_start(d(2027, 1, 2), Sunday), d(2026, 12, 27));
    }

    #[test]
    fn elapsed_is_graded() {
        use chrono::{Local, TimeZone};
        let at = |y, m, day, h, min| {
            Local
                .with_ymd_and_hms(y, m, day, h, min, 0)
                .single()
                .unwrap()
                .timestamp_millis()
        };
        let now = at(2026, 11, 12, 15, 0);
        assert_eq!(elapsed_at(now - 30_000, now), "gerade eben");
        assert_eq!(elapsed_at(at(2026, 11, 12, 14, 55), now), "vor 5 Min.");
        assert_eq!(elapsed_at(at(2026, 11, 12, 12, 0), now), "vor 3 Std.");
        assert_eq!(elapsed_at(at(2026, 11, 11, 9, 0), now), "gestern");
        assert_eq!(elapsed_at(at(2026, 11, 8, 15, 0), now), "vor 4 Tagen");
        assert_eq!(elapsed_at(at(2026, 11, 3, 9, 0), now), "3. Nov.");
    }

    #[test]
    fn next_monday_is_strictly_after() {
        assert_eq!(next_monday(d(2026, 11, 9)), d(2026, 11, 16)); // Montag
        assert_eq!(next_monday(d(2026, 11, 15)), d(2026, 11, 16)); // Sonntag
    }
}
