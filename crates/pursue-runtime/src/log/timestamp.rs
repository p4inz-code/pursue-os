//! Dependency-free UTC timestamp formatting for log records.

/// Formats a Unix timestamp (seconds) as an RFC 3339 UTC string, e.g.
/// `1970-01-01T00:00:00Z`.
///
/// Deterministic and testable; avoids a datetime dependency for the single
/// representation the logging foundation needs.
pub fn rfc3339_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Converts days since the Unix epoch to a `(year, month, day)` civil date.
///
/// Implements Howard Hinnant's `civil_from_days` algorithm, correct for the
/// full proleptic Gregorian calendar.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::rfc3339_from_unix;

    #[test]
    fn epoch() {
        assert_eq!(rfc3339_from_unix(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn one_day_after_epoch() {
        assert_eq!(rfc3339_from_unix(86_400), "1970-01-02T00:00:00Z");
    }

    #[test]
    fn known_timestamps() {
        // 2021-01-01T00:00:00Z
        assert_eq!(rfc3339_from_unix(1_609_459_200), "2021-01-01T00:00:00Z");
        // 2023-11-14T22:13:20Z
        assert_eq!(rfc3339_from_unix(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn leap_day() {
        // 2000-02-29T00:00:00Z (leap year)
        assert_eq!(rfc3339_from_unix(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn century_boundary() {
        // 2000-01-01T00:00:00Z
        assert_eq!(rfc3339_from_unix(946_684_800), "2000-01-01T00:00:00Z");
        // 1999-12-31T23:59:59Z
        assert_eq!(rfc3339_from_unix(946_684_799), "1999-12-31T23:59:59Z");
    }
}
