//! Dates without a date library: days since 1970-01-01 (proleptic Gregorian calendar).

/// Days since 1970-01-01 of a date (H. Hinnant, "chrono-compatible low-level date
/// algorithms").
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date of a number of days since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

pub fn format_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// RFC 3339 in UTC, e.g. 2026-11-24T10:30:00Z.
pub fn format_datetime(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let s = secs.rem_euclid(86_400);
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        format_date(days),
        s / 3600,
        s % 3600 / 60,
        s % 60
    )
}

/// Reads `2026-11-24` (days since 1970).
pub fn parse_date(text: &str) -> Option<i64> {
    let t = text.trim();
    let mut parts = t.splitn(3, '-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if t.len() != 10 || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// Reads a date (start of the day) or `2026-11-24T10:30:00` with optional `Z` or offset
/// (seconds since 1970, UTC).
pub fn parse_datetime(text: &str) -> Option<i64> {
    let t = text.trim();
    if t.len() == 10 {
        return parse_date(t).map(|d| d * 86_400);
    }
    let (date, rest) = (t.get(..10)?, t.get(11..)?);
    if !matches!(t.as_bytes().get(10), Some(b'T' | b't' | b' ')) {
        return None;
    }
    let days = parse_date(date)?;
    let (time, offset) = match rest.find(['Z', 'z', '+', '-']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let time = time.split('.').next()?;
    let mut hms = time.split(':');
    let h: i64 = hms.next()?.parse().ok()?;
    let mi: i64 = hms.next()?.parse().ok()?;
    let s: i64 = hms.next().map_or(Some(0), |s| s.parse().ok())?;
    if h > 23 || mi > 59 || s > 59 {
        return None;
    }
    let off = match offset {
        "" | "Z" | "z" => 0,
        o => {
            let sign = if o.starts_with('-') { -1 } else { 1 };
            let (oh, om) = o[1..].split_once(':')?;
            sign * (oh.parse::<i64>().ok()? * 3600 + om.parse::<i64>().ok()? * 60)
        }
    };
    Some(days * 86_400 + h * 3600 + mi * 60 + s - off)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Today (UTC), in days since 1970.
pub fn today() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86_400) as i64
}

/// The same day `years` years earlier (29 February becomes 28 February).
pub fn years_before(days: i64, years: u32) -> i64 {
    let (y, m, d) = civil_from_days(days);
    let y = y - i64::from(years);
    days_from_civil(y, m, d.min(days_in_month(y, m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for d in [-800_000, -1, 0, 1, 11_016, 20_416, 2_932_896] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(parse_date("2024-02-29"), Some(19_782));
        assert_eq!(parse_date("2023-02-29"), None);
        assert_eq!(parse_datetime("1970-01-02T01:00:00+01:00"), Some(86_400));
        assert_eq!(format_datetime(86_400 + 61), "1970-01-02T00:01:01Z");
        assert_eq!(format_date(years_before(19_782, 1)), "2023-02-28");
    }
}
