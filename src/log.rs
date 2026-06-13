//! Minimal logging: timestamped lines `<UTC timestamp> [INSTANCE_ID] <message>`.
//!
//! Timestamps are rendered in **UTC** from `std::time::SystemTime` with no external
//! crate. This differs from the original Node.js tool, which logged local time; UTC
//! is preferred for servers and avoids a whole class of timezone bugs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Random 4-digit id, fixed for the life of the process (mirrors the JS `INSTANCE_ID`).
pub fn instance_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| format!("{:04}", rand_u64() % 10_000))
}

/// Fresh random 6-digit id for a single connection (mirrors the JS `sessionId`).
pub fn session_id() -> String {
    format!("{:06}", rand_u64() % 1_000_000)
}

/// Emit one log line. Build the whole line first so a single `println!` keeps it
/// atomic even under many concurrent tasks.
pub fn emit(args: std::fmt::Arguments<'_>) {
    println!("{} [{}] {}", timestamp(), instance_id(), args);
}

/// `logln!("...", ...)` — the ergonomic front door to [`emit`].
#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => { $crate::log::emit(format_args!($($arg)*)) };
}

/// Current UTC time as `YYYY-MM-DD HH:MM:SS.mmm`.
pub fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let millis = now.subsec_millis();
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        y,
        mo,
        d,
        tod / 3_600,
        (tod % 3_600) / 60,
        tod % 60,
        millis
    )
}

/// Group an integer with thousands separators (mirrors the JS `Intl.NumberFormat`),
/// e.g. `1234567` -> `"1,234,567"`.
pub fn group(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, ch) in s.char_indices() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Convert a count of days since the Unix epoch into `(year, month, day)`.
/// Howard Hinnant's `civil_from_days` algorithm — valid for the whole range we care about.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

/// Non-cryptographic random `u64` with no external crate: a splitmix64 stream seeded
/// once from wall-clock nanoseconds and the PID, advanced by an atomic counter. Good
/// enough for cosmetic instance/session ids.
fn rand_u64() -> u64 {
    static SEED: OnceLock<u64> = OnceLock::new();
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seed = *SEED.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678_9abc_def0);
        (nanos ^ ((std::process::id() as u64).wrapping_shl(17))) | 1
    });
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    // splitmix64
    let mut z = seed.wrapping_add(n.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1)); // Unix epoch
        assert_eq!(civil_from_days(18_993), (2022, 1, 1)); // 2022-01-01
                                                           // 2026-06-13: days from civil = 20617
        assert_eq!(civil_from_days(20_617), (2026, 6, 13));
    }

    #[test]
    fn timestamp_shape() {
        let ts = timestamp();
        // "YYYY-MM-DD HH:MM:SS.mmm" == 23 chars
        assert_eq!(ts.len(), 23, "got {ts:?}");
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], " ");
        assert_eq!(&ts[19..20], ".");
    }

    #[test]
    fn group_separators() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(1_000), "1,000");
        assert_eq!(group(1_234_567), "1,234,567");
    }

    #[test]
    fn ids_are_padded() {
        assert_eq!(instance_id().len(), 4);
        assert_eq!(session_id().len(), 6);
    }
}
