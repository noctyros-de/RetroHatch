// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros (Rust translation and integration)
// Derived from maragotchi/tamaemu/src/periph.c; see UPSTREAM.json and THIRD_PARTY_LICENSES.md.
//! S1C33 peripherals with disconnected IR; no simulated peer or RTC acceleration.
use super::profile::HardwareProfile;
const DIV: [u32; 8] = [1, 2, 4, 16, 64, 256, 1024, 4096];
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timer {
    pub counter: u16,
    pub accum: u32,
    pub a_fired: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Peripherals {
    pub io: Vec<u8>,
    pub timers: [Timer; 6],
    pub sccr: u32,
    pub pllc: u32,
    pub opt: u32,
    pub unlocked: bool,
    pub mclk: f64,
    pub osc3: f64,
    pub cycles: u64,
    pub seconds: f64,
    pub rtc_fraction: f64,
    pub rtc_irq_fraction: f64,
    pub rtc_seconds: u64,
    pub pending: u32,
    pub level: u8,
    pub wake: bool,
    pub buttons: u8,
    pub ir_ready_at: u64,
    pub irq_count: u64,
    pub scan_serial: u64,
    pub tone_on: bool,
    pub tone_frequency: f64,
    pub phase: f64,
    pub sample_fraction: f64,
    pub pcm: Vec<i16>,
    pub tone_events: u64,
    pub t2_frequency: f64,
    pub t2_duty: f64,
    pub t2_attacks: u64,
}
impl Peripherals {
    pub fn new(p: HardwareProfile) -> Self {
        Self {
            io: vec![0; 8192],
            timers: std::array::from_fn(|_| Timer::default()),
            sccr: 0,
            pllc: 0,
            opt: 0,
            unlocked: false,
            mclk: p.osc3_hz as f64,
            osc3: p.osc3_hz as f64,
            cycles: 0,
            seconds: 0.0,
            rtc_fraction: 0.0,
            rtc_irq_fraction: 0.0,
            rtc_seconds: 0,
            pending: 0,
            level: 0,
            wake: false,
            buttons: 0,
            ir_ready_at: 0,
            irq_count: 0,
            scan_serial: 0,
            tone_on: false,
            tone_frequency: 0.0,
            phase: 0.0,
            sample_fraction: 0.0,
            pcm: vec![],
            tone_events: 0,
            t2_frequency: 0.0,
            t2_duty: 0.5,
            t2_attacks: 0,
        }
    }
    fn byte(&self, a: u32) -> u8 {
        self.io[(a - 0x300000) as usize]
    }
    fn set(&mut self, a: u32, v: u8) {
        self.io[(a - 0x300000) as usize] = v;
    }
    fn word(&self, a: u32) -> u16 {
        u16::from_le_bytes([self.byte(a), self.byte(a + 1)])
    }
    fn raw32(&self, a: u32) -> u32 {
        u32::from_le_bytes(
            self.io[(a - 0x300000) as usize..(a - 0x300000) as usize + 4]
                .try_into()
                .unwrap(),
        )
    }
    fn store32(&mut self, a: u32, v: u32) {
        self.io[(a - 0x300000) as usize..(a - 0x300000) as usize + 4]
            .copy_from_slice(&v.to_le_bytes());
    }
    fn recalc(&mut self) {
        let src = (self.sccr >> 2) & 3;
        self.mclk = if src == 1 {
            32768.0
        } else if src == 3 && self.pllc & 1 != 0 {
            self.osc3 / (((self.sccr >> 20) & 15) + 1) as f64 * (((self.pllc >> 4) & 15) + 1) as f64
        } else {
            self.osc3 / (1u32 << ((self.sccr >> 8) & 7)) as f64
        };
        if self.sccr & 0x1000 != 0 {
            self.mclk /= 2.0;
        }
    }
    fn cmu_write(&mut self, a: u32, v: u32) {
        let off = a - 0x301b00;
        if off == 0x24 {
            self.unlocked = v & 255 == 0x96;
            self.store32(a, v);
            return;
        }
        if off <= 0x14 && !self.unlocked {
            return;
        }
        self.store32(a, v);
        match off {
            8 => {
                self.sccr = v;
                self.recalc()
            }
            12 => {
                self.pllc = v;
                self.recalc()
            }
            20 => self.opt = v,
            _ => {}
        }
    }
    fn consider(&mut self, vector: u32, level: u8) {
        self.wake = true;
        if self.pending == 0 || level > self.level {
            self.pending = vector;
            self.level = level;
        }
    }
    pub fn scan(&mut self) {
        self.scan_serial = self.scan_serial.wrapping_add(1);
        self.pending = 0;
        self.level = 0;
        self.wake = false;
        for p in 0..4 {
            if self.byte(0x300270) & self.byte(0x300280) & (1 << p) != 0 {
                let l = (self.byte(0x300260 + p / 2) >> (if p & 1 == 1 { 4 } else { 0 })) & 7;
                self.consider(16 + p, l);
            }
        }
        for x in 0..6 {
            let active = self.byte(0x300272 + x / 2) & self.byte(0x300282 + x / 2);
            let b = if x & 1 == 1 { 6 } else { 2 };
            let level = (self.byte(0x300266 + x / 2) >> (if x & 1 == 1 { 4 } else { 0 })) & 7;
            for (bit, v) in [(b, 30 + x * 4), (b + 1, 31 + x * 4)] {
                if active & (1 << bit) != 0 {
                    self.consider(v, level);
                }
            }
        }
        let active = self.byte(0x300277) & self.byte(0x300287);
        if active & 4 != 0 {
            self.consider(65, ((self.byte(0x30026a) >> 4) & 7).max(1));
        }
        if active & 2 != 0 {
            self.consider(64, (self.byte(0x30026a) & 7).max(1));
        }
        if active & 8 != 0 {
            self.consider(68, 4);
        }
        if self.byte(0x3002a6) & self.byte(0x3002a9) & 16 != 0 {
            self.consider(88, 4);
        }
        for b in 3..=5 {
            if self.byte(0x300276) & self.byte(0x300286) & (1 << b) != 0 {
                self.consider(57 + b, 4);
            }
        }
    }
    pub fn buttons(&mut self, mask: u8) {
        let edge = mask & !self.buttons & 7;
        self.buttons = mask & 7;
        if edge != 0 {
            self.set(0x300280, self.byte(0x300280) | edge);
            self.scan();
        }
    }
    pub fn read8(&mut self, a: u32, p: HardwareProfile) -> u8 {
        if a == 0x300380 {
            let mut v = (7 | if !p.debug_strap_high { 64 } else { 0 }) & !self.buttons;
            if p.ir_gpio {
                v |= 1 << p.ir_gpio_rx_bit;
            }
            return v;
        }
        let v = self.byte(a);
        if a == p.ir_base + 1 {
            self.set(p.ir_base + 2, self.byte(p.ir_base + 2) & !1);
        }
        if a == p.ir_base + 2 {
            return if self.cycles >= self.ir_ready_at {
                v | 2
            } else {
                v & !2
            };
        }
        v
    }
    pub fn read32(&mut self, a: u32, p: HardwareProfile) -> u32 {
        let mut v = self.raw32(a);
        if a == 0x301714 {
            v &= !0x40;
        }
        if a == 0x301700 {
            self.set(p.ir_base + 2, self.byte(p.ir_base + 2) & !1);
        }
        v
    }
    fn tx(&mut self, p: HardwareProfile) {
        let div =
            self.byte(p.ir_base + 6) as u32 | (((self.byte(p.ir_base + 7) & 15) as u32) << 16);
        let baud = 576000.0 / (div + 1) as f64;
        let baud = if (300.0..=1e6).contains(&baud) {
            baud
        } else {
            115200.0
        };
        self.ir_ready_at = self.cycles + (self.mclk * 10.0 / baud).max(1.0) as u64;
        self.set(p.ir_base + 2, self.byte(p.ir_base + 2) & !32);
    }
    pub fn write32(&mut self, a: u32, v: u32, p: HardwareProfile) {
        if (0x301b00..=0x301b24).contains(&a) {
            self.cmu_write(a, v);
            return;
        }
        if a == 0x301900 {
            if v & 1 != 0 {
                self.set(a, self.byte(a) & !1);
            }
            return;
        }
        self.store32(a, v);
        if a == 0x301704 {
            self.tx(p);
        }
    }
    fn tone(&mut self) {
        let cra = self.word(0x300780);
        let crb = self.word(0x300782);
        let ctl = self.byte(0x300786);
        let ck = self.byte(0x3007e0);
        let on = ctl & 1 != 0 && ctl & 4 != 0 && ck & 8 != 0 && cra > 0 && cra < crb;
        let f = (self.mclk / DIV[(ck & 7) as usize] as f64 / (crb as f64 + 1.0)) as f32 as f64;
        if on != self.tone_on || on && f != self.tone_frequency {
            self.tone_events += 1;
            self.tone_on = on;
            self.tone_frequency = f;
        }
    }
    pub fn write8(&mut self, a: u32, v: u8, p: HardwareProfile) {
        if (0x301b00..=0x301b27).contains(&a) {
            let word = a & !3;
            let shift = (a & 3) * 8;
            self.cmu_write(
                word,
                (self.raw32(word) & !(255 << shift)) | ((v as u32) << shift),
            );
            return;
        }
        if (0x300280..=0x30028f).contains(&a) || a == 0x3002a9 {
            self.set(a, self.byte(a) & !v);
            self.scan();
            return;
        }
        if a == 0x301900 {
            if v & 1 != 0 {
                self.set(a, self.byte(a) & !1);
            }
            return;
        }
        if a == 0x300544 && v & 6 != 0 {
            self.set(a, v);
            self.set(0x300540, 0x50);
            self.set(0x300541, 3);
            self.set(0x300548, 0x50);
            self.set(0x300549, 3);
            self.set(0x300287, self.byte(0x300287) | 2);
            self.scan();
            return;
        }
        if (p.ir_base..=p.ir_base + 7).contains(&a) {
            self.set(a, v);
            if a == p.ir_base {
                self.tx(p);
            }
            return;
        }
        if a == 0x3007e4 && v & 8 != 0 {
            let a = self.word(0x300790);
            let b = self.word(0x300792);
            if b > 1 {
                self.t2_frequency = self.mclk / DIV[(v & 7) as usize] as f64 / (b as f64 + 1.0);
                self.t2_duty = if a > 0 && a < b {
                    a as f64 / (b as f64 + 1.0)
                } else {
                    0.5
                };
                self.t2_attacks += 1;
            }
        }
        if (0x300780..=0x3007af).contains(&a) {
            let x = ((a - 0x300780) / 8) as usize;
            match (a - 0x300780) % 8 {
                6 => {
                    if v & 2 != 0 {
                        self.timers[x].counter = 0;
                        self.timers[x].accum = 0;
                    }
                    self.set(a, v & !2);
                    if x == 0 {
                        self.tone();
                    }
                    return;
                }
                4 => {
                    self.timers[x].counter = (self.timers[x].counter & 0xff00) | v as u16;
                    self.timers[x].a_fired = false;
                }
                5 => self.timers[x].counter = (self.timers[x].counter & 255) | ((v as u16) << 8),
                _ => {}
            }
        }
        self.set(a, v);
        if (0x300270..=0x30027f).contains(&a) || a == 0x3002a6 {
            self.scan();
        }
        if (0x300780..=0x300783).contains(&a) || a == 0x3007e0 {
            self.tone();
        }
    }
    fn bcd(v: u8, max: u8) -> Option<u8> {
        let mut n = v.wrapping_add(1);
        if n & 15 > 9 {
            n = (n & 240).wrapping_add(16);
        }
        if n > max { None } else { Some(n) }
    }
    fn second(&mut self) {
        self.rtc_seconds += 1;
        for (a, max, start) in [
            (0x301910, 0x59, 0),
            (0x301914, 0x59, 0),
            (0x301918, 0x23, 0),
        ] {
            if let Some(v) = Self::bcd(self.byte(a), max) {
                self.set(a, v);
                return;
            } else {
                self.set(a, start);
            }
        }
        self.set(0x301928, (self.byte(0x301928) + 1) % 7);
        let m = self.byte(0x301920);
        let mi = ((m >> 4) & 1) * 10 + (m & 15);
        let days = [
            0, 0x31, 0x28, 0x31, 0x30, 0x31, 0x30, 0x31, 0x31, 0x30, 0x31, 0x30, 0x31,
        ];
        let max = if mi == 2 && (self.byte(0x301924) & 15) % 4 == 0 {
            0x29
        } else {
            *days.get(mi as usize).filter(|_| mi != 0).unwrap_or(&0x31)
        };
        for (a, max, start) in [(0x30191c, max, 1), (0x301920, 0x12, 1), (0x301924, 0x99, 0)] {
            if let Some(v) = Self::bcd(self.byte(a), max) {
                self.set(a, v);
                return;
            } else {
                self.set(a, start);
            }
        }
    }
    pub fn tick(&mut self, cycles: u32, p: HardwareProfile) {
        let seconds = cycles as f64 / self.mclk;
        self.seconds += seconds;
        if self.byte(0x301908) & 2 == 0 && self.byte(0x30190c) & 1 == 0 {
            self.rtc_fraction += seconds;
            while self.rtc_fraction >= 1.0 {
                self.rtc_fraction -= 1.0;
                self.second();
            }
        }
        let period = [1.0 / 64.0, 1.0, 60.0, 3600.0][((self.byte(0x301904) >> 2) & 3) as usize];
        self.rtc_irq_fraction += seconds;
        if self.rtc_irq_fraction >= period {
            self.rtc_irq_fraction -= period;
            self.set(0x301900, self.byte(0x301900) | 1);
            if self.byte(0x301904) & 1 != 0 {
                self.set(0x300287, self.byte(0x300287) | 4);
            }
        }
        if self.ir_ready_at != 0 && self.cycles >= self.ir_ready_at {
            self.ir_ready_at = 0;
            let en = self.byte(0x300276);
            let bit = if en & 32 != 0 {
                5
            } else if en & 8 != 0 {
                3
            } else {
                4
            };
            if en & (32 | 8 | 16) != 0 {
                self.set(0x300286, self.byte(0x300286) | (1 << bit));
            }
        }
        for x in 0..6 {
            let base = 0x300780 + x as u32 * 8;
            let ck = self.byte(0x3007e0 + x as u32 * 2);
            if self.byte(base + 6) & 1 == 0 || ck & 8 == 0 {
                continue;
            }
            let a = self.word(base);
            let b = self.word(base + 2);
            let div = DIV[(ck & 7) as usize];
            let t = &mut self.timers[x];
            t.accum = t.accum.wrapping_add(cycles);
            let clocks = t.accum / div;
            t.accum -= clocks * div;
            let mut flags = 0;
            for _ in 0..clocks.min(65536) {
                t.counter = t.counter.wrapping_add(1);
                if a != 0 && !t.a_fired && t.counter >= a {
                    flags |= if x & 1 == 1 { 128 } else { 8 };
                    t.a_fired = true;
                }
                if t.counter >= b {
                    flags |= if x & 1 == 1 { 64 } else { 4 };
                    t.counter = 0;
                    t.a_fired = false;
                }
            }
            let count = t.counter;
            let reg = 0x300282 + x as u32 / 2;
            self.set(reg, self.byte(reg) | flags);
            if clocks != 0 {
                self.set(base + 4, count as u8);
                self.set(base + 5, (count >> 8) as u8);
            }
        }
        self.scan();
        let _ = p;
    }
    /// Integrate samples before register/clock changes. Preserve phase through silence.
    pub fn audio_until(&mut self, cycle: u64) {
        let elapsed = cycle.saturating_sub(self.cycles) as f64 / self.mclk;
        self.sample_fraction += elapsed * 44100.0;
        let count = self.sample_fraction.floor() as usize;
        self.sample_fraction -= count as f64;
        for _ in 0..count {
            let mut sample = 0;
            if self.tone_on && (20.0..=20000.0).contains(&self.tone_frequency) {
                self.phase = (self.phase + self.tone_frequency / 44100.0).fract();
                sample = if self.phase < 0.5 { 4500 } else { -4500 };
            }
            self.pcm.push(sample);
        }
        self.cycles = cycle;
    }
}
