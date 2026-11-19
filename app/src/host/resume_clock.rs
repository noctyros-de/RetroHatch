// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Optional elapsed-UTC RTC adjustment, never CPU/gameplay catch-up.
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_ELAPSED_MS: u64 = 366 * 86_400_000;

pub fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|v| v.as_millis().try_into().ok())
        .unwrap_or(0)
}

/// Only firmware families exercised with genuine private images are enabled.
pub fn verified_tg18(code_hash: u64) -> bool {
    // Meets advances its time of day in current tests, but its date is not yet
    // correct after a skipped day. Keep it and untested On editions disabled.
    code_hash == 0x8313_24F3_B2FA_6768 // On Magic EN v058
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResumeClock {
    Disabled,
    Unsupported,
    Legacy,
    Invalid,
    Future,
    Excessive,
    Adjusted(u64),
}
impl ResumeClock {
    pub fn elapsed(saved_at: Option<u64>, now: u64) -> Self {
        let Some(saved) = saved_at else {
            return Self::Legacy;
        };
        if saved == 0 || now == 0 {
            return Self::Invalid;
        }
        let Some(elapsed) = now.checked_sub(saved) else {
            return Self::Future;
        };
        if elapsed > MAX_ELAPSED_MS {
            return Self::Excessive;
        }
        Self::Adjusted(elapsed)
    }
    pub fn message(self) -> String {
        let detail = match self {
            Self::Adjusted(ms) if ms >= 1000 => format!(
                "; clock advanced by {}h {}m {}s",
                ms / 3_600_000,
                ms / 60_000 % 60,
                ms / 1000 % 60
            ),
            Self::Legacy => "; legacy save has no clock timestamp".into(),
            Self::Invalid => "; clock sync skipped: invalid timestamp".into(),
            Self::Future => "; clock sync skipped: future save timestamp".into(),
            Self::Excessive => "; clock sync skipped: interval exceeds one year".into(),
            _ => String::new(),
        };
        format!("Resumed saved progress{detail}")
    }
}

/// Follows TG18's elapsed-RTC restore, also adjusting the already-running SoC TOD.
/// No emulated time, CPU, alarm, RAM, tick phase or gameplay state is advanced.
pub fn advance_tg18(machine: &mut tg18::Machine, elapsed_ms: u64) -> Result<(), String> {
    if elapsed_ms > MAX_ELAPSED_MS {
        return Err("Clock interval exceeds one year".into());
    }
    let ticks = (u128::from(elapsed_ms) * 32768 / 1000) as i64;
    let rtc = &machine.sys.rtc;
    let running = (machine.sys.game_time - rtc.base_time) * 32768.0;
    if !running.is_finite() || running < i64::MIN as f64 || running >= i64::MAX as f64 {
        return Err("Invalid RTC counter phase".into());
    }
    let current = rtc
        .base_ticks
        .checked_add(running as i64)
        .ok_or("RTC counter overflow")?;
    current
        .checked_add(ticks)
        .filter(|v| (0..(1i64 << 48)).contains(v))
        .ok_or("RTC counter would exceed its 48-bit range")?;
    let base = rtc
        .base_ticks
        .checked_add(ticks)
        .ok_or("RTC offset overflow")?;
    let tod = machine
        .sys
        .rtc_irq
        .tod_base
        .checked_add((elapsed_ms / 1000) as i64)
        .ok_or("RTC time-of-day overflow")?;
    let phase = machine.sys.rtc_irq.tod_set_at - (elapsed_ms % 1000) as f64 / 1000.0;
    if !phase.is_finite() {
        return Err("Invalid RTC time-of-day phase".into());
    }
    machine.sys.rtc.base_ticks = base;
    machine.sys.rtc_irq.tod_base = tod;
    machine.sys.rtc_irq.tod_set_at = phase;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn semantic_chunks(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
        use std::io::Read;
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&bytes[8..])
            .read_to_end(&mut raw)
            .unwrap();
        let mut at = 0;
        let mut chunks = std::collections::BTreeMap::new();
        while at < raw.len() {
            let n = raw[at] as usize;
            at += 1;
            let name = String::from_utf8(raw[at..at + n].to_vec()).unwrap();
            at += n;
            let n = u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()) as usize;
            at += 4;
            let mut data = raw[at..at + n].to_vec();
            at += n;
            if name == "info" {
                data.truncate(8);
            }
            chunks.insert(name, data);
        }
        chunks
    }
    #[test]
    fn only_the_firmware_with_verified_clock_and_calendar_is_enabled() {
        assert!(verified_tg18(0x8313_24F3_B2FA_6768));
        for rom in tg18::roms::KNOWN {
            assert_eq!(
                verified_tg18(rom.hash),
                rom.region == "en" && rom.version == 58
            );
        }
        assert!(!verified_tg18(0));
    }
    #[test]
    fn utc_elapsed_handles_rollover_dst_rollback_and_invalid_times_without_local_time() {
        let saved = 1_700_000_000_000;
        for hours in [0, 17, 23, 24, 25, 48, 24 * 365] {
            assert_eq!(
                ResumeClock::elapsed(Some(saved), saved + hours * 3_600_000),
                ResumeClock::Adjusted(hours * 3_600_000)
            );
        }
        // UTC boundaries covering Berlin spring/fall DST and year rollover.
        assert_eq!(
            ResumeClock::elapsed(Some(1774695600000), 1774778400000),
            ResumeClock::Adjusted(23 * 3_600_000)
        );
        assert_eq!(
            ResumeClock::elapsed(Some(1792836000000), 1792926000000),
            ResumeClock::Adjusted(25 * 3_600_000)
        );
        assert_eq!(
            ResumeClock::elapsed(Some(1798758000000), 1798819200000),
            ResumeClock::Adjusted(17 * 3_600_000)
        );
        assert_eq!(ResumeClock::elapsed(None, saved), ResumeClock::Legacy);
        assert_eq!(ResumeClock::elapsed(Some(0), saved), ResumeClock::Invalid);
        assert_eq!(ResumeClock::elapsed(Some(saved), 0), ResumeClock::Invalid);
        assert_eq!(
            ResumeClock::elapsed(Some(saved), saved - 1),
            ResumeClock::Future
        );
        assert_eq!(
            ResumeClock::elapsed(Some(1), MAX_ELAPSED_MS + 2),
            ResumeClock::Excessive
        );
    }
    #[test]
    fn tg18_adjustment_is_atomic_preserves_user_offset_and_executes_nothing() {
        let image = std::sync::Arc::new(vec![0; tg18::FLASH_SIZE]);
        let mut machine = tg18::Machine::new(image.clone(), &image, 123456.25);
        machine.sys.rtc_irq.tod_base = 21 * 3600;
        machine.sys.rtc_irq.fired = 8;
        machine.sys.rtc.regs[0x40] = 6;
        machine.sys.rtc.regs[0x20] = 17;
        let before = tg18::snapshot::Snapshot::capture(&machine).to_bytes();
        let before_ticks = machine.sys.rtc.ticks(machine.sys.game_time);
        advance_tg18(&mut machine, 17 * 3_600_000 + 125).unwrap();
        assert_eq!(
            machine.sys.rtc.ticks(machine.sys.game_time),
            before_ticks + 17 * 3600 * 32768 + 4096
        );
        assert_eq!(machine.sys.rtc_irq.tod(machine.sys.game_time), 14 * 3600);
        assert_eq!(machine.executed, 0);
        assert_eq!(machine.sys.game_time, 0.0);
        // Undo only the changed clock bases; the entire emulated snapshot must match.
        machine.sys.rtc.base_ticks = before_ticks;
        machine.sys.rtc_irq.tod_base = 21 * 3600;
        machine.sys.rtc_irq.tod_set_at = 0.0;
        let after = tg18::snapshot::Snapshot::capture(&machine).to_bytes();
        // Snapshot 'info' contains wall-clock capture time, so compare restored state chunks.
        assert_eq!(semantic_chunks(&before), semantic_chunks(&after));
        machine.sys.rtc.base_ticks = (1i64 << 48) - 1;
        let old = machine.sys.rtc_irq.tod_base;
        assert!(advance_tg18(&mut machine, 1000).is_err());
        assert_eq!(machine.sys.rtc.base_ticks, (1i64 << 48) - 1);
        assert_eq!(machine.sys.rtc_irq.tod_base, old);
    }
}
