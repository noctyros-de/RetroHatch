//! P1 E0C6200 register/pin/timer behavior derived from BrickEmuPy (CC0).
use super::buzzer::Buzzer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Port {
    K0,
    K1,
    R0,
    R1,
    R2,
    R3,
    R4,
    P0,
    P1,
    P2,
    P3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Io {
    registers: [u8; 0x7F],
    low: [u8; 11],
    high: [u8; 11],
    latch: [u8; 11],
    timer: u8,
    stopwatch: [u8; 2],
    programmable: u8,
    reload: u8,
    serial: u8,
    timer_phase: i32,
    stopwatch_phase_bits: u64,
    programmable_phase: i32,
    pub buzzer: Buzzer,
}

impl Default for Io {
    fn default() -> Self {
        let mut registers = [0; 0x7F];
        registers[0x41] = 15;
        registers[0x71] = 8;
        registers[0x73] = 8;
        Self {
            registers,
            low: [0; 11],
            high: [0; 11],
            latch: [0; 11],
            timer: 0,
            stopwatch: [0; 2],
            programmable: 0,
            reload: 0,
            serial: 0,
            timer_phase: 0,
            stopwatch_phase_bits: 0,
            programmable_phase: 0,
            buzzer: Buzzer::default(),
        }
    }
}

impl Io {
    pub fn supports(address: u16) -> bool {
        matches!(address, 0xF00..=0xF05 | 0xF10..=0xF15 | 0xF20..=0xF27
            | 0xF30..=0xF31 | 0xF40..=0xF42 | 0xF50..=0xF54
            | 0xF60..=0xF63 | 0xF70..=0xF7B | 0xF7D..=0xF7E)
    }
    pub fn fast_clock(&self) -> bool {
        self.registers[0x70] & 8 != 0
    }
    pub fn lcd_control(&self) -> u8 {
        self.registers[0x71]
    }
    pub fn port(&self, port: Port) -> u8 {
        let i = port as usize;
        let (direction, pullup, pushpull) = match i {
            0..=1 => (0, 15, 0),
            2..=6 => (15, 0, 0),
            _ => {
                let bit = 1 << (i - 7);
                (
                    if self.registers[0x7D] & bit != 0 {
                        15
                    } else {
                        0
                    },
                    if self.registers[0x7E] & bit == 0 {
                        15
                    } else {
                        0
                    },
                    15,
                )
            }
        };
        ((!direction & (self.high[i] | (pullup & !self.low[i])))
            | (direction & self.latch[i] & (pullup | pushpull)))
            & 15
    }
    /// level=None releases the external driver, allowing the profile pull-up to act.
    pub fn pin(&mut self, port: Port, mask: u8, level: Option<bool>) {
        let mask = mask & 15;
        let old = self.port(port) & mask;
        let i = port as usize;
        self.low[i] &= !mask;
        self.high[i] &= !mask;
        match level {
            Some(false) => self.low[i] |= mask,
            Some(true) => self.high[i] |= mask,
            None => {}
        }
        let new = self.port(port) & mask;
        if port == Port::K0
            && (old & self.registers[0x14]) != (new & self.registers[0x14])
            && (self.registers[0x41] & mask) != new
        {
            self.registers[4] |= 1;
            if mask & 8 != 0 {
                self.process_programmable();
            }
        }
        if port == Port::K1
            && (old & self.registers[0x15]) != (new & self.registers[0x15])
            && mask != new
        {
            self.registers[5] |= 1;
        }
    }
    pub fn read(&mut self, address: u16) -> u8 {
        let index = (address - 0xF00) as usize;
        match index {
            0..=5 => {
                let value = self.registers[index];
                self.registers[index] = 0;
                value
            }
            0x20 => self.timer & 15,
            0x21 => self.timer >> 4,
            0x22 => self.stopwatch[0],
            0x23 => self.stopwatch[1],
            0x24 => self.programmable & 15,
            0x25 => self.programmable >> 4,
            0x26 => self.reload & 15,
            0x27 => self.reload >> 4,
            0x30 => self.serial & 15,
            0x31 => self.serial >> 4,
            0x40 => self.port(Port::K0),
            0x42 => self.port(Port::K1),
            0x50..=0x54 => self.latch[index - 0x50 + 2],
            0x60 => self.port(Port::P0),
            0x61 => self.port(Port::P1),
            0x62 => self.port(Port::P2),
            0x63 => self.port(Port::P3),
            0x73 | 0x76 | 0x7A | 0x7B => 0,
            0x75 => self.registers[0x75] | if self.buzzer.ringing() { 8 } else { 0 },
            _ => self.registers[index],
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        let v = value & 15;
        let i = (address - 0xF00) as usize;
        match i {
            0..=5 | 0x20..=0x25 | 0x40 | 0x42 | 0x73 | 0x7A | 0x7B => {}
            0x11 => self.registers[i] = v & 3,
            0x12 | 0x13 => self.registers[i] = v & 1,
            0x26 => self.reload = (self.reload & 0xF0) | v,
            0x27 => self.reload = (self.reload & 15) | (v << 4),
            0x30 => self.serial = (self.serial & 0xF0) | v,
            0x31 => self.serial = (self.serial & 15) | (v << 4),
            0x50..=0x54 => {
                self.latch[i - 0x50 + 2] = v;
                if i == 0x54 {
                    self.buzzer.set_on(v & 8 == 0);
                }
            }
            0x60..=0x63 => self.latch[i - 0x60 + 7] = v,
            0x74 => {
                self.registers[i] = v;
                self.buzzer.set_frequency(v & 7);
            }
            0x75 => {
                self.registers[i] = v & 3;
                self.buzzer.control(v, self.registers[0x74] & 8 != 0);
            }
            0x76 => {
                if v & 2 != 0 {
                    self.timer = 0;
                }
            }
            0x77 => {
                if v & 2 != 0 {
                    self.stopwatch = [0; 2];
                }
                self.registers[i] = v & 1;
            }
            0x78 => {
                if v & 2 != 0 {
                    self.programmable = self.reload;
                }
                self.registers[i] = v & 1;
            }
            _ => self.registers[i] = v,
        }
    }
    pub fn pending_vector(&self) -> Option<u8> {
        let r = &self.registers;
        if r[2] & r[0x12] != 0 {
            Some(0xC)
        } else if r[3] & r[0x13] != 0 {
            Some(0xA)
        } else if r[5] != 0 {
            Some(8)
        } else if r[4] != 0 {
            Some(6)
        } else if r[1] & r[0x11] != 0 {
            Some(4)
        } else if r[0] & r[0x10] != 0 {
            Some(2)
        } else {
            None
        }
    }
    pub fn clock(&mut self) {
        self.buzzer.clock();
        let selection = (self.registers[0x79] & 7) as usize;
        if selection > 1 {
            self.programmable_phase -= 1;
            if self.programmable_phase <= 0 {
                self.programmable_phase += [0, 0, 128, 64, 32, 16, 8, 4][selection];
                self.process_programmable();
            }
        }
        // Preserve the reference's binary64 327.68 reload and its boundary rounding.
        // Storing IEEE bits keeps machine snapshots and equality deterministic.
        let mut stopwatch_phase = f64::from_bits(self.stopwatch_phase_bits) - 1.0;
        if stopwatch_phase <= 0.0 {
            stopwatch_phase += 32768.0 / 100.0;
            if self.registers[0x77] & 1 != 0 {
                self.stopwatch[0] = (self.stopwatch[0] + 1) % 10;
                if self.stopwatch[0] == 0 {
                    self.stopwatch[1] = (self.stopwatch[1] + 1) % 10;
                    self.registers[1] |= 1;
                    if self.stopwatch[1] == 0 {
                        self.registers[1] |= 2;
                    }
                }
            }
        }
        self.stopwatch_phase_bits = stopwatch_phase.to_bits();
        self.timer_phase -= 1;
        if self.timer_phase <= 0 {
            self.timer_phase += 128;
            let next = self.timer.wrapping_add(1);
            for (bit, flag) in [(4, 1), (16, 2), (64, 4), (128, 8)] {
                if next & bit < self.timer & bit {
                    self.registers[0] |= flag;
                }
            }
            self.timer = next;
        }
    }
    fn process_programmable(&mut self) {
        // This deliberately follows the reference, which does not gate on PTRUN.
        self.programmable = self.programmable.wrapping_sub(1);
        if self.programmable == 0 {
            self.programmable = self.reload;
            self.registers[2] |= 1;
        }
        if self.registers[0x79] & 8 != 0 {
            self.latch[Port::R3 as usize] ^= 8;
        }
    }
}

impl Io {
    pub(crate) fn encode_state(&self, e: &mut crate::core::state::Encoder) {
        e.bytes(&self.registers);
        e.bytes(&self.low);
        e.bytes(&self.high);
        e.bytes(&self.latch);
        e.u8(self.timer);
        e.bytes(&self.stopwatch);
        e.u8(self.programmable);
        e.u8(self.reload);
        e.u8(self.serial);
        e.i32(self.timer_phase);
        e.u64(self.stopwatch_phase_bits);
        e.i32(self.programmable_phase);
        self.buzzer.encode_state(e);
    }
    pub(crate) fn decode_state(
        &mut self,
        d: &mut crate::core::state::Decoder<'_>,
    ) -> crate::core::state::Result<()> {
        self.registers = d.array::<127>()?;
        self.low = d.array::<11>()?;
        self.high = d.array::<11>()?;
        self.latch = d.array::<11>()?;
        self.timer = d.u8()?;
        self.stopwatch = d.array::<2>()?;
        self.programmable = d.u8()?;
        self.reload = d.u8()?;
        self.serial = d.u8()?;
        self.timer_phase = d.i32()?;
        self.stopwatch_phase_bits = d.u64()?;
        self.programmable_phase = d.i32()?;
        self.buzzer.decode_state(d)?;
        use crate::core::state::require;
        require(
            self.registers
                .iter()
                .chain(&self.low)
                .chain(&self.high)
                .chain(&self.latch)
                .all(|&v| v <= 15)
                && self.stopwatch.iter().all(|&v| v < 10),
            "Invalid peripheral registers",
        )?;
        require(
            self.low.iter().zip(self.high).all(|(&a, b)| a & b == 0),
            "Invalid external pin levels",
        )?;
        require(
            (0..=128).contains(&self.timer_phase) && (0..=128).contains(&self.programmable_phase),
            "Invalid timer phase",
        )?;
        let phase = f64::from_bits(self.stopwatch_phase_bits);
        require(
            phase.is_finite() && (0.0..=328.0).contains(&phase),
            "Invalid stopwatch phase",
        )?;
        Ok(())
    }
}
