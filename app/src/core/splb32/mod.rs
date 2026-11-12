// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros; original Rust integration.
//! Native SPLB32, ported directly from azya52/BrickEmuPy (CC0-1.0).
//! No code from the paused Tama-Go/asterick implementation is used.
mod cpu;
pub mod eeprom;
pub mod lcd;
mod state;
use crate::core::state::{Result, require};
use std::sync::Arc;
pub const HZ: u64 = 4_000_000;
const SUB_DIV: f64 = 4_000_000.0 / 32768.0;
#[derive(Clone)]
pub struct Machine {
    pub pc: i64,
    pub a: i64,
    pub x: i64,
    pub y: i64,
    pub sp: i64,
    pub nf: i64,
    pub vf: i64,
    pub bf: i64,
    pub df: i64,
    pub iflag: i64,
    pub zf: i64,
    pub cf: i64,
    pub ram: [i64; 1216],
    pub dpram: [i64; 512],
    pub rom: Arc<Vec<u8>>,
    pub cycles: f64,
    pub instructions: u64,
    pub interrupts: u64,
    rom_bank: i64,
    cpu_enbl: i64,
    cpu_div: i64,
    clk32768_enable: i64,
    lcd_ctrl1: i64,
    int_ctrl: i64,
    ireq: i64,
    nmi_ctrl: i64,
    nmireq: i64,
    wakeup_ctrl: i64,
    wakeupreq: i64,
    time_base: i64,
    timer_ctrl: i64,
    t1khz: i64,
    tm0: f64,
    tm1: f64,
    tm0_reload: i64,
    tm1_reload: i64,
    tm0_counter: f64,
    tm1_counter: f64,
    t1khz_counter: f64,
    audio_ch0_ctrl: i64,
    audio_ch0_data: i64,
    audio_ch1_ctrl: i64,
    audio_ch1_data: i64,
    byte_mirror: i64,
    nibble_swap: i64,
    pdir: [i64; 4],
    attr: [i64; 4],
    platch: [i64; 4],
    pin0: [i64; 4],
    pin1: [i64; 4],
    addr_irq: i64,
    addr_nmi: i64,
    pub ee: eeprom::Eeprom,
    envelope: [f64; 2],
    toggle: [f64; 2],
    dac: [f64; 2],
    audio_at: f64,
    audio_pos: f64,
    audio_acc: f64,
    pcm: Vec<i16>,
}
pub fn validate(bytes: &[u8]) -> Result<()> {
    require(
        bytes.len() == 524288,
        "Connection V3 requires a complete 512-KiB SPLB32 ROM",
    )?;
    let reset = u16::from_le_bytes([bytes[0x7ffc], bytes[0x7ffd]]);
    require(reset >= 0xc000, "Invalid Connection V3 reset vector")
}
impl Machine {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        validate(bytes)?;
        Ok(Self {
            pc: u16::from_le_bytes([bytes[0x7ffc], bytes[0x7ffd]]) as i64,
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            nf: 0,
            vf: 0,
            bf: 0,
            df: 0,
            iflag: 1,
            zf: 0,
            cf: 0,
            ram: [0; 1216],
            dpram: [0; 512],
            rom: Arc::new(bytes.to_vec()),
            cycles: 0.0,
            instructions: 0,
            interrupts: 0,
            rom_bank: 0,
            cpu_enbl: 1,
            cpu_div: 2,
            clk32768_enable: 128,
            lcd_ctrl1: 0,
            int_ctrl: 0,
            ireq: 0,
            nmi_ctrl: 0,
            nmireq: 0,
            wakeup_ctrl: 0,
            wakeupreq: 0,
            time_base: 0,
            timer_ctrl: 0,
            t1khz: 0,
            tm0: 0.,
            tm1: 0.,
            tm0_reload: 0,
            tm1_reload: 0,
            tm0_counter: 0.,
            tm1_counter: 0.,
            t1khz_counter: 0.,
            audio_ch0_ctrl: 0,
            audio_ch0_data: 0,
            audio_ch1_ctrl: 0,
            audio_ch1_data: 0,
            byte_mirror: 0,
            nibble_swap: 0,
            pdir: [0; 4],
            attr: [0; 4],
            platch: [0; 4],
            pin0: [0, 4, 0, 0],
            pin1: [0; 4],
            addr_irq: u16::from_le_bytes([bytes[0x7ffe], bytes[0x7fff]]) as i64,
            addr_nmi: u16::from_le_bytes([bytes[0x7ffa], bytes[0x7ffb]]) as i64,
            ee: eeprom::Eeprom::default(),
            envelope: [0.; 2],
            toggle: [1.; 2],
            dac: [0.; 2],
            audio_at: 0.,
            audio_pos: 0.,
            audio_acc: 0.,
            pcm: Vec::new(),
        })
    }
    pub fn reset(&mut self) {
        let ee = self.ee.clone();
        let cycles = self.cycles;
        let instructions = self.instructions;
        let interrupts = self.interrupts;
        let mut fresh = Self::new(&self.rom).expect("Validated ROM");
        fresh.ee = ee;
        fresh.cycles = cycles;
        fresh.audio_at = cycles;
        fresh.instructions = instructions;
        fresh.interrupts = interrupts;
        *self = fresh;
    }
    pub fn physical_pc(&self) -> i64 {
        if self.pc >= 0xc000 {
            self.pc & 0x7fff
        } else {
            (self.pc & 0x7fff) | self.rom_bank
        }
    }
    pub fn display(&self) -> [u8; 512] {
        if self.clk32768_enable != 0 {
            if self.lcd_ctrl1 & 13 == 1 {
                return self.dpram.map(|v| v as u8);
            }
            if self.lcd_ctrl1 & 4 != 0 {
                return [255; 512];
            }
        }
        [0; 512]
    }
    fn port_read(&self, p: usize) -> i64 {
        let pull = if p == 3 { 6 } else { 0 };
        ((!self.pdir[p] & (self.pin1[p] | (pull & !self.pin0[p])))
            | (self.pdir[p] & self.platch[p] & (pull | !self.attr[p])))
            & 255
    }
    fn pin(&mut self, p: usize, mask: i64, level: i64) {
        let prev = self.port_read(p);
        self.pin0[p] &= !mask;
        self.pin1[p] &= !mask;
        if level == 0 {
            self.pin0[p] |= mask;
        }
        if level == 1 {
            self.pin1[p] |= mask;
        }
        if p == 2 && mask & 2 != 0 && (prev & 16) > (self.port_read(p) & 16) {
            self.ireq |= 1;
        }
        if p == 0 && self.port_read(p) & mask != 0 {
            self.wakeupreq |= 1;
        }
    }
    pub fn button_mask(&self) -> u8 {
        self.pin1[0] as u8 & 7
    }
    pub fn buttons(&mut self, mask: u8) {
        for bit in [1, 2, 4] {
            self.pin(0, bit, i64::from(mask as i64 & bit != 0));
        }
    }
    fn emit_port(&mut self, p: usize) {
        if p == 3 {
            let mask = self.port_read(3) as u8;
            if let Some(level) = self.ee.update(mask) {
                self.pin(3, 2, level);
            }
        }
    }
    pub fn read_mem(&self, a: i64) -> i64 {
        let a = a as usize;
        match a {
            0xc000..=0xffff => self.rom[a & 0x7fff] as i64,
            0x4000..=0xbfff => self.rom[(a & 0x7fff) | self.rom_bank as usize] as i64,
            0x3e00..=0x3fff => self.dpram[a - 0x3e00],
            0x40..=0x4ff => self.ram[a - 0x40],
            _ => match a {
                0 => self.rom_bank >> 15,
                4 => self.cpu_div,
                8 => self.port_read(0),
                9 => self.port_read(3),
                10 => self.port_read(1),
                11 => self.port_read(2),
                1 => self.ireq,
                0x38 => self.nmireq | (self.nmi_ctrl & 128),
                0x18 => self.wakeupreq,
                12 => self.t1khz & 255, // Expose the low byte on the eight-bit CPU bus.
                15 => self.timer_ctrl,
                16 => self.tm0 as i64 & 255,
                17 => self.tm0 as i64 >> 8,
                20 => self.tm1 as i64 & 255,
                21 => self.tm1 as i64 >> 8,
                0x24 => self.lcd_ctrl1,
                0x3e => self.byte_mirror,
                0x3f => self.nibble_swap,
                18 => self.audio_ch0_ctrl,
                19 => 0, // Channel 0 data is write-only in BrickEmuPy.
                22 => self.audio_ch1_ctrl,
                23 => self.audio_ch1_data,
                _ => 0,
            },
        }
    }
    pub fn write_mem(&mut self, a: i64, v: i64) {
        let a = a as usize;
        if (0x40..0x500).contains(&a) {
            self.ram[a - 0x40] = v;
            return;
        }
        if (0x3e00..0x4000).contains(&a) {
            self.dpram[a - 0x3e00] = v;
            return;
        }
        match a {
            0 => self.rom_bank = (v & 15) << 15,
            4 => {
                if v & 7 == 7 {
                    self.cpu_enbl = 0;
                } else {
                    self.cpu_div = v & 7;
                }
            }
            0x0e => self.clk32768_enable = v & 128,
            6 | 7 | 8 | 0x28 | 0x29 | 10 | 0x2a | 0x2b | 11 | 0x2c | 0x2d | 9 => {
                let (p, kind) = match a {
                    6 => (0, 0),
                    7 => (0, 1),
                    8 => (0, 2),
                    0x28 => (1, 0),
                    0x29 => (1, 1),
                    10 => (1, 2),
                    0x2a => (2, 0),
                    0x2b => (2, 1),
                    11 => (2, 2),
                    0x2c => (3, 0),
                    0x2d => (3, 1),
                    _ => (3, 2),
                };
                let value = if p == 1 && kind != 2 {
                    (v & 1) * 3 | (v & 2) * 6 | (v & 4) * 12 | (v & 8) * 24
                } else if p == 3 {
                    v & 63
                } else {
                    v
                };
                match kind {
                    0 => self.attr[p] = value,
                    1 => self.pdir[p] = value,
                    _ => self.platch[p] = value,
                }
                self.emit_port(p);
            }
            1 => {
                self.int_ctrl = v;
                self.ireq = 0;
            }
            2 => self.ireq &= !v,
            0x38 => {
                self.nmi_ctrl = v;
                self.nmireq &= v;
            }
            0x18 => {
                self.wakeup_ctrl = v;
                self.wakeupreq &= v;
            }
            12 => self.time_base = v,
            15 => self.timer_ctrl = v,
            16 => self.tm0_reload = (self.tm0_reload & 0xff00) | v,
            17 => {
                self.tm0_reload = (self.tm0_reload & 255) | (v << 8);
                self.tm0 = self.tm0_reload as f64;
            }
            20 => self.tm1_reload = (self.tm1_reload & 0xff00) | v,
            21 => {
                self.tm1_reload = (self.tm1_reload & 255) | (v << 8);
                self.tm1 = self.tm1_reload as f64;
            }
            0x24 => self.lcd_ctrl1 = v,
            0x3e => self.byte_mirror = (v as u8).reverse_bits() as i64,
            0x3f => self.nibble_swap = (v as u8).rotate_left(4) as i64,
            18 => self.audio_ch0_ctrl = v,
            22 => self.audio_ch1_ctrl = v,
            19 => {
                self.audio_ch0_data = v;
                self.set_audio(0, self.audio_ch0_ctrl, v);
            }
            23 => {
                self.audio_ch1_data = v;
                self.set_audio(1, self.audio_ch1_ctrl, v);
            }
            _ => {}
        }
    }
    fn set_audio(&mut self, c: usize, ctrl: i64, data: i64) {
        self.render_audio();
        if ctrl & 64 != 0 {
            self.envelope[c] = (data & 255) as f64 / 255.;
            if data == 0 {
                self.dac[c] = 0.;
            }
        } else {
            self.dac[c] = if data == 0 {
                0.
            } else {
                (data & 255) as f64 / 128. - 1.
            };
        }
    }
    fn toggle_audio(&mut self, timer: i64) {
        for c in 0..2 {
            let ctrl = if c == 0 {
                self.audio_ch0_ctrl
            } else {
                self.audio_ch1_ctrl
            };
            if ctrl & timer != 0 && ctrl & 64 != 0 && self.envelope[c] != 0. {
                self.render_audio();
                self.toggle[c] *= -1.;
                self.dac[c] = self.toggle[c] * self.envelope[c];
            }
        }
    }
    fn timers(&mut self, cycles: f64) {
        for t in 0..2 {
            let mode = if t == 0 {
                self.timer_ctrl & 0x60
            } else {
                self.timer_ctrl & 3
            };
            if mode == 0 {
                continue;
            }
            let mut counter = if t == 0 {
                self.tm0_counter
            } else {
                self.tm1_counter
            };
            let mut value = if t == 0 { self.tm0 } else { self.tm1 };
            let reload = if t == 0 {
                self.tm0_reload
            } else {
                self.tm1_reload
            };
            counter -= cycles;
            // Reference TODO modes would not advance their counter. Fail before executing
            // them in clock(), rather than entering an unbounded loop or inventing behavior.
            while counter <= 0. {
                if mode == if t == 0 { 0x20 } else { 1 } {
                    counter = 1.;
                    value += cycles;
                } else {
                    counter += SUB_DIV;
                    value += 1.;
                }
                while value > 65535. {
                    value -= 65536. - reload as f64;
                    self.ireq |= if t == 0 { 16 } else { 32 };
                    self.wakeupreq |= if t == 0 { 4 } else { 16 };
                    if t == 1 {
                        self.nmireq |= 2;
                        if self.nmi_ctrl & 2 != 0 {
                            self.nmi();
                        }
                    }
                    self.toggle_audio(if t == 0 { 8 } else { 16 });
                }
            }
            if t == 0 {
                self.tm0_counter = counter;
                self.tm0 = value;
            } else {
                self.tm1_counter = counter;
                self.tm1 = value;
            }
        }
        self.t1khz_counter -= cycles;
        while self.t1khz_counter <= 0. {
            self.t1khz_counter += SUB_DIV * 32.;
            self.t1khz += 1;
            if self.t1khz & [7, 1, 3, 0][(self.time_base & 3) as usize] == 0 {
                self.ireq |= 8;
                self.wakeupreq |= 8;
            }
            if self.t1khz & [511, 127, 255, 63][((self.time_base & 12) >> 2) as usize] == 0 {
                self.ireq |= 4;
                self.wakeupreq |= 2;
            }
        }
    }
    pub fn clock(&mut self) -> Result<f64> {
        require(
            self.timer_ctrl & 0x60 != 0x60 && self.timer_ctrl & 3 != 3,
            "SPLB32 external/cascaded timer source is unimplemented in the reference",
        )?;
        let cycles;
        if self.cpu_enbl != 0 {
            cycles = (self.step_cpu()? << self.cpu_div) as f64;
            self.instructions += 1;
            require(
                self.timer_ctrl & 0x60 != 0x60 && self.timer_ctrl & 3 != 3,
                "SPLB32 external/cascaded timer source is unimplemented in the reference",
            )?;
            self.timers(cycles);
            if self.iflag == 0 && self.ireq & self.int_ctrl != 0 {
                self.irq();
                self.interrupts += 1;
            }
        } else {
            cycles = SUB_DIV;
            if self.wakeup_ctrl & self.wakeupreq != 0 {
                self.cpu_enbl = 1;
                self.cpu_div = 2;
            }
            if self.clk32768_enable != 0 {
                self.timers(cycles);
            }
        }
        self.cycles += cycles;
        Ok(cycles)
    }
    pub fn advance_cycles(&mut self, target: f64) -> Result<()> {
        while self.cycles < target {
            self.clock()?;
        }
        self.render_audio();
        Ok(())
    }
    fn render_audio(&mut self) {
        let mut span = (self.cycles - self.audio_at) * 44100.;
        self.audio_at = self.cycles;
        let dac = (self.dac[0] + self.dac[1]) * 0.5;
        while self.audio_pos + span >= HZ as f64 {
            let take = HZ as f64 - self.audio_pos;
            self.audio_acc += dac * take;
            self.pcm
                .push((self.audio_acc * 8192. / HZ as f64).clamp(-32768., 32767.) as i16);
            span -= take;
            self.audio_pos = 0.;
            self.audio_acc = 0.;
        }
        self.audio_pos += span;
        self.audio_acc += dac * span;
    }
    pub fn drain_audio(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.pcm)
    }
}

#[cfg(test)]
mod tests;
