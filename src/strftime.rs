//! Safe replacements for the libc time functions used by the Date plugin:
//! `localtime_r`/`gmtime_r`, `mktime` and locale-aware `strftime`.

use jiff::{civil::DateTime, tz::TimeZone, Span, Timestamp, Zoned};
use pure_rust_locales::{locale_match, Locale};

/// `localtime(epoch)`, or with `gmt` the fields of `gmtime(epoch)`.
///
/// Perl's `POSIX::strftime` treats the fields it is given as local time, so
/// `strftime($fmt, gmtime($t))` shows UTC wall-clock fields but the local
/// zone for `%z`, `%Z` and `%s`.  The Date plugin relies on that, so the gmt
/// case reinterprets the UTC fields in the local zone the same way.  (One
/// divergence: during DST Perl's `%s` here comes from `mktime` with
/// `isdst = 0`, which is off by the DST shift; this returns the true epoch.)
pub fn from_epoch(epoch: i64, gmt: bool) -> Zoned {
    let ts = Timestamp::from_second(epoch).unwrap_or(Timestamp::UNIX_EPOCH);
    let local = ts.to_zoned(TimeZone::system());
    if !gmt {
        return local;
    }
    ts.to_zoned(TimeZone::UTC)
        .datetime()
        .to_zoned(TimeZone::system())
        .unwrap_or(local)
}

/// `mktime()` on broken-down local time with `tm_isdst = -1`.  Like mktime,
/// out-of-range fields are normalised (month 13 is January of the next year,
/// day 0 is the last day of the previous month, and so on).  Returns `None`
/// if the result is outside the representable range.
pub fn mktime_local(
    year: i32,
    mon: i32,
    mday: i32,
    hour: i32,
    min: i32,
    sec: i32,
) -> Option<Zoned> {
    let months = year as i64 * 12 + (mon as i64 - 1);
    let (y, m) = (months.div_euclid(12), months.rem_euclid(12) + 1);
    let base = DateTime::new(i16::try_from(y).ok()?, m as i8, 1, 0, 0, 0, 0).ok()?;
    let offset = Span::new()
        .try_days(mday as i64 - 1)
        .ok()?
        .try_hours(hour as i64)
        .ok()?
        .try_minutes(min as i64)
        .ok()?
        .try_seconds(sec as i64)
        .ok()?;
    // jiff's default ("compatible") disambiguation matches mktime: the
    // earlier instant for a repeated hour, skipping forward over a gap.
    base.checked_add(offset)
        .ok()?
        .to_zoned(TimeZone::system())
        .ok()
}

/// Map a POSIX locale name (`fr_FR`, `de_DE.UTF-8`, `en_GB@euro`); `C`
/// and `POSIX` map to the C locale.
fn parse_locale(name: &str) -> Option<Locale> {
    let base = name.split(['.', '@']).next().unwrap_or("");
    Locale::try_from(base).ok()
}

/// Locale-dependent conversions, or `None` for ones that don't use the locale.
fn localized(conv: char, z: &Zoned, loc: Locale) -> Option<String> {
    let wday = z.weekday().to_sunday_zero_offset() as usize;
    let mon = z.month() as usize - 1;
    let am_pm = || {
        let names = locale_match!(loc => LC_TIME::AM_PM);
        names
            .get(usize::from(z.hour() >= 12))
            .copied()
            .unwrap_or("")
            .to_string()
    };
    let expand = |fmt: &str| strftime_in(fmt, z, loc);
    match conv {
        'a' => Some(locale_match!(loc => LC_TIME::ABDAY)[wday].to_string()),
        'A' => Some(locale_match!(loc => LC_TIME::DAY)[wday].to_string()),
        'b' | 'h' => Some(locale_match!(loc => LC_TIME::ABMON)[mon].to_string()),
        'B' => Some(locale_match!(loc => LC_TIME::MON)[mon].to_string()),
        'p' => Some(am_pm()),
        'P' => Some(am_pm().to_lowercase()),
        'c' => Some(expand(locale_match!(loc => LC_TIME::D_T_FMT))),
        'x' => Some(expand(locale_match!(loc => LC_TIME::D_FMT))),
        'X' => Some(expand(locale_match!(loc => LC_TIME::T_FMT))),
        'r' => Some(expand(locale_match!(loc => LC_TIME::T_FMT_AMPM))),
        _ => None,
    }
}

/// Numeric/zone conversions glibc supports, delegated to jiff.
const GLIBC_NUMERIC: &str = "CdDeFgGHIjklmMnRsStTuUVwWyYzZ%";

/// C-compatible `strftime`.  Conversions glibc does not know are copied
/// through verbatim.  `locale` is a POSIX name such as `fr_FR.UTF-8`
/// (unknown or absent means the C locale).
pub fn strftime(fmt: &str, z: &Zoned, locale: Option<&str>) -> String {
    let loc = locale.and_then(parse_locale).unwrap_or(Locale::POSIX);
    strftime_in(fmt, z, loc)
}

fn strftime_in(fmt: &str, z: &Zoned, loc: Locale) -> String {
    let mut out = String::new();
    let mut chars = fmt.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // %[flags][width][E|O]conv
        let mut upper = false;
        while let Some(&(_, f)) = chars.peek() {
            if matches!(f, '-' | '_' | '0' | '^' | '#') || f.is_ascii_digit() {
                upper |= f == '^';
                chars.next();
            } else {
                break;
            }
        }
        if let Some(&(_, 'E' | 'O')) = chars.peek() {
            chars.next();
        }
        let Some((end, conv)) = chars.next() else {
            out.push_str(&fmt[start..]);
            break;
        };
        let raw = &fmt[start..end + conv.len_utf8()];
        let piece = if let Some(s) = localized(conv, z, loc) {
            s
        } else if GLIBC_NUMERIC.contains(conv) {
            // jiff has no E/O modifiers; glibc ignores them outside special locales.
            let spec = raw.replace("%E", "%").replace("%O", "%");
            match jiff::fmt::strtime::format(spec.as_str(), z) {
                Ok(s) => s,
                Err(_) => raw.to_string(),
            }
        } else {
            raw.to_string()
        };
        out.push_str(&if upper { piece.to_uppercase() } else { piece });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_c_strftime() {
        let z = from_epoch(0, true);
        assert_eq!(
            strftime("%Y-%m-%d %H:%M:%S %%", &z, None),
            "1970-01-01 00:00:00 %"
        );
        assert_eq!(
            strftime("%A %b %-d %^a %Ey %Q", &z, None),
            "Thursday Jan 1 THU 70 %Q"
        );
        assert_eq!(strftime("%A %B", &z, Some("fr_FR.UTF-8")), "jeudi janvier");
        assert_eq!(
            strftime("%c|%x|%X|%r|%D|%T|%e", &z, None),
            "Thu Jan  1 00:00:00 1970|01/01/70|00:00:00|12:00:00 AM|01/01/70|00:00:00| 1"
        );
    }

    #[test]
    fn mktime_normalises() {
        let a = mktime_local(2024, 13, 1, 0, 0, 0).unwrap();
        let b = mktime_local(2025, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(a, b);
        let c = mktime_local(2024, 3, 0, 0, 0, 0).unwrap();
        assert_eq!(strftime("%m-%d", &c, None), "02-29");
    }
}
