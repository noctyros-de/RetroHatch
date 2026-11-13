// SPDX-License-Identifier: GPL-3.0-only
//! GPLB52320A backend, ported/adapted from mameise/Paramix-Studio T7Emu.
//! The upstream core credits asterick/tamago (BSD; exact notice remains under review).
mod cpu;
pub mod figure;
mod state;
use super::state::require;
pub use cpu::Cpu;
use std::sync::Arc;
pub const HZ: u64 = 4_000_000;
#[derive(Clone)]
pub struct Machine {
    pub cpu: Cpu,
    pub ram: [u8; 1536],
    pub dram: [u8; 512],
    pub reg: [u8; 256],
    pub cycles: u64,
    pub instructions: u64,
    pub interrupts: u64,
    pub halted: bool,
    pub rom: Arc<Vec<u8>>,
    pub figure: Option<Arc<figure::Figure>>,
    pub spi_reads: u64,
    figure_ready: u64,
    key_active: u64,
    bank: u8,
    keys: u8,
    video: u64,
    tbh: u64,
    timer_due: [u64; 2],
    timer_period: [u64; 2],
    spi_cs: bool,
    spi_pos: u32,
    spi_addr: u32,
    spi_cmd: u8,
    spi_fifo: [u8; 256],
    spi_w: u8,
    spi_r: u8,
    ee: Eeprom,
    audio_at: u64,
    audio_pos: u64,
    audio_acc: i64,
    dac: i32,
    pcm: Vec<i16>,
}
#[derive(Clone)]
struct Eeprom {
    data: [u8; 4096],
    state: u8,
    output: u8,
    clk: u8,
    line: u8,
    shift: u8,
    bits: u8,
    address: u16,
    addressbyte: u8,
}
impl Default for Eeprom {
    fn default() -> Self {
        Self {
            data: [0; 4096],
            state: 0,
            output: 1,
            clk: 0,
            line: 0,
            shift: 0,
            bits: 0,
            address: 0,
            addressbyte: 0,
        }
    }
}
impl Eeprom {
    fn update(&mut self, power: bool, clk: bool, data: bool) {
        let clk = u8::from(clk);
        let data = u8::from(data);
        let cd = clk as i8 - self.clk as i8;
        let dd = data as i8 - self.line as i8;
        self.clk = clk;
        self.line = data;
        if !power {
            self.state = 0;
            self.output = 1;
            return;
        }
        if cd == 0 && dd == 0 {
            return;
        }
        if clk != 0 && dd != 0 {
            if dd > 0 {
                self.state = 0;
                self.output = 0;
            } else {
                self.state = 1;
                self.output = 0;
                self.bits = 0;
                self.shift = 0;
            }
        }
        if self.state == 0 {
            return;
        }
        if cd > 0 {
            self.shift = self.shift.wrapping_shl(1) | data;
        } else if cd < 0 {
            if self.bits < 8 {
                self.output = if self.state == 3 {
                    u8::from(
                        self.data[self.address as usize].wrapping_shl(self.bits as u32) & 128 != 0,
                    )
                } else {
                    1
                };
            } else {
                self.output = 0;
                match self.state {
                    1 => match self.shift & 0xf1 {
                        0xa0 => {
                            self.state = 2;
                            self.addressbyte = 0;
                            self.address = 0;
                        }
                        0xa1 => self.state = 3,
                        _ => self.output = 1,
                    },
                    2 => {
                        self.address = self.address.wrapping_shl(8) | self.shift as u16;
                        self.addressbyte += 1;
                        if self.addressbyte >= 2 {
                            self.state = 4;
                            self.address &= 4095;
                        }
                    }
                    4 => {
                        self.data[self.address as usize] = self.shift;
                        self.address = (self.address + 1) & 4095;
                    }
                    3 => self.address = (self.address + 1) & 4095,
                    _ => {}
                }
            }
            self.bits = (self.bits + 1) % 9;
        }
    }
}
pub fn validate(bytes: &[u8]) -> Result<(), String> {
    require(
        bytes.len() == 655360,
        "Tama-Go requires a 640-KiB main firmware image; figure and debug cartridges are accessories",
    )?;
    let pc = u16::from_le_bytes([bytes[0x3ffc], bytes[0x3ffd]]);
    require(
        pc >= 0xc000 && bytes[1..3] != [0xaa, 0x55],
        "Invalid Tama-Go reset vector or accessory image",
    )
}
impl Machine {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        validate(bytes)?;
        let mut m = Self {
            cpu: Cpu::default(),
            ram: [0; 1536],
            dram: [0; 512],
            reg: [0; 256],
            cycles: 0,
            instructions: 0,
            interrupts: 0,
            halted: false,
            rom: Arc::new(bytes.to_vec()),
            figure: None,
            spi_reads: 0,
            figure_ready: 0,
            key_active: 0,
            bank: 0,
            keys: 15,
            video: HZ / 60,
            tbh: HZ / 2,
            timer_due: [0; 2],
            timer_period: [0; 2],
            spi_cs: true,
            spi_pos: 0,
            spi_addr: 0,
            spi_cmd: 0,
            spi_fifo: [0; 256],
            spi_w: 0,
            spi_r: 0,
            ee: Eeprom::default(),
            audio_at: 0,
            audio_pos: 0,
            audio_acc: 0,
            dac: 0,
            pcm: Vec::new(),
        };
        m.cpu.pc = m.read16(0xfffc);
        Ok(m)
    }
    pub fn reset(&mut self) {
        self.cpu = Cpu::default();
        self.cpu.pc = self.read16(0xfffc);
        self.halted = false;
        self.keys = 15;
    }
    pub fn eeprom(&self) -> &[u8; 4096] {
        &self.ee.data
    }
    pub fn button_mask(&self) -> u8 {
        !self.keys & 7
    }
    pub fn buttons(&mut self, mask: u8) {
        let keys = !mask & 15;
        if self.keys != keys {
            self.key_active = self.cycles + HZ;
            self.halted = false;
        }
        self.keys = keys;
        if mask & 7 != 0 && self.reg[7] & 1 != 0 {
            self.reg[8] |= 1;
            self.halted = false;
        }
    }
    pub fn set_figure(&mut self, figure: Option<Arc<figure::Figure>>) {
        self.figure = figure;
        self.figure_ready = self.cycles + HZ * 5 / 2;
        self.spi_cs = true;
        self.spi_pos = 0;
        self.spi_w = 0;
        self.spi_r = 0;
    }
    pub fn vector(&self, i: usize) -> u16 {
        u16::from_le_bytes([self.rom[0x3fc0 + i * 2], self.rom[0x3fc1 + i * 2]])
    }
    pub fn read16(&mut self, a: u16) -> u16 {
        let l = self.read(a);
        u16::from_le_bytes([l, self.read(a.wrapping_add(1))])
    }
    pub fn read(&mut self, a: u16) -> u8 {
        match a {
            0..=0xfff => self.ram[a as usize % 1536],
            0x1000..=0x2fff => self.dram[a as usize & 511],
            0x3000..=0x3fff => {
                let r = a as u8 as usize;
                match r {
                    0x12 => {
                        let mask = self.reg[0x11];
                        let value = self.reg[0x12];
                        let id = if self.figure.is_some()
                            && self.cycles >= self.figure_ready
                            && mask & value & 16 == 0
                        {
                            1
                        } else {
                            0
                        };
                        (mask & value) | (!mask & (self.keys | id << 5))
                    }
                    0x16 => {
                        (self.reg[0x15] & self.reg[0x16])
                            | if self.ee.output != 0 {
                                !self.reg[0x15]
                            } else {
                                0
                            }
                    }
                    0x82 => self.reg[0x80] >> 4,
                    0x83 => self.reg[0x80] & 15,
                    0x8a => self.reg[0x80].rotate_left(4),
                    0xb6 => {
                        if self.spi_r == self.spi_w {
                            255
                        } else {
                            let v = self.spi_fifo[self.spi_r as usize];
                            self.spi_r = self.spi_r.wrapping_add(1);
                            v
                        }
                    }
                    0xb7 if self.figure.is_some() && self.cycles >= self.figure_ready => 4,
                    _ => self.reg[r],
                }
            }
            0x4000..=0xbfff => self.rom[self.bank as usize * 32768 + (a as usize - 0x4000)],
            _ => self.rom[a as usize - 0xc000],
        }
    }
    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0..=0xfff => self.ram[a as usize % 1536] = v,
            0x1000..=0x2fff => self.dram[a as usize & 511] = v,
            0x3000..=0x3fff => {
                let r = a as u8 as usize;
                if r == 0x73 || r == 0x74 {
                    self.reg[r] &= !v;
                    return;
                }
                self.render_audio();
                self.reg[r] = v;
                match r {
                    0 => self.bank = v % 20,
                    1 if v == 7 => {
                        if self.keys & 7 == 7 && self.cycles >= self.key_active {
                            self.halted = true;
                        } else {
                            self.reg[8] |= 1;
                        }
                    }
                    0x15 | 0x16 => {
                        if r == 0x16 {
                            let cs = v & 16 != 0;
                            if !cs && self.spi_cs {
                                self.spi_pos = 0;
                                self.spi_addr = 0;
                                self.spi_w = 0;
                                self.spi_r = 0;
                            }
                            self.spi_cs = cs;
                        }
                        let d = !self.reg[0x15] | self.reg[0x16];
                        self.ee.update(d & 4 != 0, d & 2 != 0, d & 1 != 0);
                    }
                    0xb3 => self.spi_tx(v),
                    0x30..=0x35 => self.recalc_timers(r),
                    0x60 | 0x65 => {
                        self.dac = if self.reg[0x65] & 2 != 0 {
                            self.reg[0x60] as i8 as i32
                        } else {
                            0
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn spi_tx(&mut self, v: u8) {
        if self.spi_cs {
            return;
        }
        let p = self.spi_pos;
        if p == 0 {
            self.spi_cmd = v;
        }
        if (1..=3).contains(&p) {
            self.spi_addr = (self.spi_addr << 8) | v as u32;
        }
        let rx = if self.spi_cmd == 3 {
            if p < 4 {
                0
            } else {
                let a = self.spi_addr as usize + (p - 4) as usize;
                self.spi_reads += 1;
                self.figure
                    .as_ref()
                    .filter(|_| self.cycles >= self.figure_ready)
                    .map(|f| f.bytes.get(a).copied().unwrap_or(0))
                    .unwrap_or(255)
            }
        } else {
            v
        };
        self.spi_fifo[self.spi_w as usize] = rx;
        self.spi_w = self.spi_w.wrapping_add(1);
        self.spi_pos = self.spi_pos.saturating_add(1);
    }
    fn irq(&mut self, i: usize) {
        let mask = u16::from_be_bytes([self.reg[0x70], self.reg[0x71]]);
        if mask & (0x8000 >> i) != 0 {
            self.reg[0x73 + i / 8] |= 0x80 >> (i % 8);
            self.halted = false;
        }
    }
    fn recalc_timers(&mut self, reg: usize) {
        for t in 0..2 {
            let src = if t == 0 {
                self.reg[0x31] >> 4
            } else {
                self.reg[0x31] & 15
            };
            let pre = u16::from_le_bytes([self.reg[0x32 + t * 2], self.reg[0x33 + t * 2]]);
            let hz = if t == 0 {
                8_000_000
            } else if self.reg[0x30] & 1 != 0 {
                524288
            } else {
                32768
            };
            let period = (((65536 - pre as u64).max(16) * HZ) << 16) / hz;
            let was = self.timer_period[t];
            self.timer_period[t] = if src == 2 { period } else { 0 };
            let restart = if t == 0 {
                reg == 0x33
            } else {
                reg == 0x35 || reg == 0x30
            };
            if was == 0 || restart {
                self.timer_due[t] = (self.cycles << 16) + period;
            }
        }
    }
    pub fn advance_cycles(&mut self, target: u64) -> Result<(), String> {
        while self.cycles < target {
            let mut next = self.video.min(self.tbh);
            for t in 0..2 {
                if self.timer_period[t] > 0 {
                    next = next.min(self.timer_due[t].div_ceil(65536));
                }
            }
            let c = if self.halted {
                next.min(target).saturating_sub(self.cycles).max(1) as u32
            } else {
                self.step_cpu()?
            };
            self.cycles += c as u64;
            for t in 0..2 {
                if self.timer_period[t] > 0 {
                    while self.timer_due[t] <= (self.cycles << 16) {
                        self.timer_due[t] += self.timer_period[t];
                        self.irq(if t == 0 { 0 } else { 12 });
                    }
                }
            }
            if self.cycles >= self.video {
                self.video += HZ / 60;
                self.irq(10);
                if self.reg[0x76] & 2 != 0 {
                    let a = self.read16(0xfffa);
                    self.interrupt(a, true);
                }
            }
            if self.cycles >= self.tbh {
                self.tbh += HZ / 2;
                self.irq(13);
            }
        }
        self.render_audio();
        Ok(())
    }
    fn render_audio(&mut self) {
        let now = self.cycles * 44100;
        let mut span = now - self.audio_at;
        self.audio_at = now;
        while self.audio_pos + span >= HZ {
            let take = HZ - self.audio_pos;
            self.audio_acc += self.dac as i64 * take as i64;
            self.pcm.push((self.audio_acc * 128 / HZ as i64) as i16);
            span -= take;
            self.audio_pos = 0;
            self.audio_acc = 0;
        }
        self.audio_pos += span;
        self.audio_acc += self.dac as i64 * span as i64;
    }
    pub fn drain_audio(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.pcm)
    }
    pub fn pixels(&self) -> Vec<u8> {
        let mut out = vec![0; 48 * 31];
        for y in 0..31 {
            let off = if y < 16 {
                0xc0 + y * 12
            } else {
                0xb4 - (y - 16) * 12
            };
            for x in 0..48 {
                out[y * 48 + x] = (self.dram[off + x / 4] >> (6 - 2 * (x % 4))) & 3;
            }
        }
        out
    }
    pub fn icons(&self) -> [u8; 10] {
        [
            self.dram[4] & 3,
            self.dram[5] >> 6,
            (self.dram[5] >> 4) & 3,
            (self.dram[5] >> 2) & 3,
            self.dram[5] & 3,
            self.dram[6] >> 6,
            (self.dram[6] >> 4) & 3,
            (self.dram[6] >> 2) & 3,
            self.dram[6] & 3,
            self.dram[7] >> 6,
        ]
    }
    pub fn rgb(&self) -> Vec<u8> {
        self.pixels()
            .into_iter()
            .flat_map(|p| {
                let v = [224, 160, 96, 32][p as usize];
                [v, v, v]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn synthetic() -> Machine {
        let mut b = vec![0xea; 655360];
        b[0x3ffc..0x3ffe].copy_from_slice(&0xc100u16.to_le_bytes());
        Machine::new(&b).unwrap()
    }
    #[test]
    fn long_running_devices_keep_loadable_clock_states() {
        let mut m = synthetic();
        m.cycles = HZ * 3600 * 24;
        m.video = m.cycles + HZ / 60;
        m.tbh = m.cycles + HZ / 2;
        m.audio_at = m.cycles * 44100;
        let s = m.save_state();
        assert_eq!(m.restored(&s).unwrap().save_state(), s);
    }
    #[test]
    fn all_reference_opcodes_execute_and_reserved_encodings_are_atomic() {
        let mut count = 0;
        for opcode in 0..=255u8 {
            let mut m = synthetic();
            m.cpu.pc = 0x200;
            m.ram[0x200] = opcode;
            m.ram[0x201] = 0xea;
            m.ram[0x202] = 0xea;
            let before = m.save_state();
            if m.step_cpu().is_ok() {
                count += 1;
            } else {
                assert_eq!(m.save_state(), before);
            }
        }
        assert_eq!(count, 151);
    }
    #[test]
    fn cpu_alu_stack_bank_and_wrapping_addressing() {
        let mut m = synthetic();
        m.cpu.pc = 0x200;
        m.ram[0x200..0x20c].copy_from_slice(&[
            0xa9, 0xff, 0x18, 0x69, 1, 0x48, 0xa9, 7, 0x68, 0x85, 0x20, 0xea,
        ]);
        for _ in 0..7 {
            m.step_cpu().unwrap();
        }
        assert_eq!(m.cpu.a, 0);
        assert_eq!(m.cpu.p & 3, 3);
        m.step_cpu().unwrap();
        assert_eq!(m.ram[0x20], 0);
        m.write(0x3000, 39);
        assert_eq!(m.bank, 19);
        m.write(0x800, 88);
        assert_eq!(m.read(0x200), 88);
        m.write(0x1200, 42);
        assert_eq!(m.read(0x1000), 42);
    }
    #[test]
    fn unsupported_instruction_has_no_partial_cpu_or_memory_effects() {
        let mut m = synthetic();
        m.ram[0x200] = 2;
        m.cpu.pc = 0x200;
        let before = m.save_state();
        assert!(m.step_cpu().unwrap_err().contains("Unsupported"));
        assert_eq!(before, m.save_state());
    }
    #[test]
    fn serial_figure_read_echo_wrap_and_live_removal() {
        let mut m = synthetic();
        let mut b = vec![0; 524288];
        b[1..3].copy_from_slice(&[170, 85]);
        b[17..21].copy_from_slice(&[170, 85, 3, 14]);
        let f = Arc::new(figure::Figure::decode(&b).unwrap());
        m.set_figure(Some(f));
        m.figure_ready = 0;
        m.write(0x3016, 0);
        for v in [3, 0, 0, 1, 0, 0] {
            m.write(0x30b3, v);
        }
        assert_eq!(
            (0..6).map(|_| m.read(0x30b6)).collect::<Vec<_>>(),
            [0, 0, 0, 0, 170, 85]
        );
        assert_eq!(m.spi_reads, 2);
        let r = m.restored(&m.save_state()).unwrap();
        assert_eq!(m.save_state(), r.save_state());
        let before = m.ee.data;
        m.set_figure(None);
        assert_eq!(m.ee.data, before);
        assert!(m.figure.is_none());
    }
    #[test]
    fn full_state_and_timer_interrupt_continuation_are_exact() {
        let mut m = synthetic();
        m.write(0x3071, 0x20);
        m.cpu.p |= 4;
        m.advance_cycles(HZ / 2).unwrap();
        assert_ne!(m.reg[0x74] & 0x20, 0);
        m.drain_audio();
        let state = m.save_state();
        let mut r = m.restored(&state).unwrap();
        m.advance_cycles(HZ).unwrap();
        r.advance_cycles(HZ).unwrap();
        assert_eq!(m.save_state(), r.save_state());
        let mut corrupt = state.clone();
        corrupt[100] ^= 1;
        assert!(m.restored(&corrupt).is_err());
    }
    #[test]
    fn display_row_order_and_all_intensities_are_preserved() {
        let mut m = synthetic();
        m.dram[0xc0] = 0x1b;
        assert_eq!(&m.pixels()[..4], &[0, 1, 2, 3]);
        m.dram[0xb4] = 0xe4;
        assert_eq!(&m.pixels()[16 * 48..16 * 48 + 4], &[3, 2, 1, 0]);
        assert_eq!(m.rgb().len(), 48 * 31 * 3);
    }
    #[test]
    fn debug_accessory_is_not_main_firmware() {
        let mut b = vec![0; 524288];
        b[1..3].copy_from_slice(&[170, 85]);
        b[17..21].copy_from_slice(&[170, 85, 3, 254]);
        assert!(figure::Figure::decode(&b).unwrap().debug);
        assert!(validate(&b).is_err());
    }
}
