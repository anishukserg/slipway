//! Время событий: UTC с точностью до секунды, `ГГГГ-ММ-ДДTЧЧ:ММ:ССZ`. Такой
//! формат упорядочивается сравнением строк, и свёртке не нужен календарь.

use std::time::{SystemTime, UNIX_EPOCH};

/// Строка — время события в формате журнала.
pub fn is_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    if separators.iter().any(|&(at, sep)| bytes[at] != sep) {
        return false;
    }
    let number = |from: usize, to: usize| -> Option<u32> {
        let part = text.get(from..to)?;
        part.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    matches!(
        (
            number(5, 7),
            number(8, 10),
            number(11, 13),
            number(14, 16),
            number(17, 19),
            number(0, 4)
        ),
        (
            Some(1..=12),
            Some(1..=31),
            Some(0..=23),
            Some(0..=59),
            Some(0..=60),
            Some(_)
        )
    )
}

/// Текущее время в формате журнала.
pub fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    format_unix(seconds)
}

/// Секунды от 1970-01-01T00:00:00Z в формате журнала.
pub fn format_unix(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX / 2);
    let rest = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Дни от 1970-01-01 — григорианская дата (алгоритм Говарда Хиннанта).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_seconds_become_utc_dates() {
        assert_eq!(format_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(
            format_unix(1_789_084_800 + 3 * 3600 + 15 * 60 + 7),
            "2026-09-11T03:15:07Z"
        );
        assert!(is_timestamp(&now()));
    }

    #[test]
    fn only_the_journal_format_is_a_timestamp() {
        assert!(is_timestamp("2026-09-11T03:15:00Z"));
        for text in [
            "2026-09-11 03:15:00Z",
            "2026-13-11T03:15:00Z",
            "2026-09-11T24:00:00Z",
            "2026-09-11T03:15:00+03:00",
            "26-09-11T03:15:00Z",
        ] {
            assert!(!is_timestamp(text), "{text}");
        }
    }
}
