// SPDX-License-Identifier: GPL-3.0-only
//! Rust port of Paramix-Studio T7Emu's 6502 interpreter, derived from asterick/tamago.
//! See developer/TAMAGO.md and THIRD_PARTY_LICENSES.md for provenance and limitations.
use super::Machine;
const C: u8 = 1;
const Z: u8 = 2;
const I: u8 = 4;
const D: u8 = 8;
const V: u8 = 64;
const N: u8 = 128;
#[derive(Clone, Debug)]
pub struct Cpu {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub p: u8,
}
impl Default for Cpu {
    fn default() -> Self {
        Self {
            pc: 0,
            a: 0,
            x: 0,
            y: 0,
            s: 255,
            p: 0x24,
        }
    }
}
impl Machine {
    fn flag(&mut self, f: u8, v: bool) {
        self.cpu.p = (self.cpu.p & !f) | if v { f } else { 0 };
    }
    fn nz(&mut self, v: u8) {
        self.flag(N, v & 128 != 0);
        self.flag(Z, v == 0);
    }
    fn push(&mut self, v: u8) {
        self.write(0x100 | self.cpu.s as u16, v);
        self.cpu.s = self.cpu.s.wrapping_sub(1);
    }
    fn pull(&mut self) -> u8 {
        self.cpu.s = self.cpu.s.wrapping_add(1);
        self.read(0x100 | self.cpu.s as u16)
    }
    fn fetch(&mut self) -> u8 {
        let v = self.read(self.cpu.pc);
        self.cpu.pc = self.cpu.pc.wrapping_add(1);
        v
    }
    fn word(&mut self) -> u16 {
        let lo = self.fetch();
        u16::from_le_bytes([lo, self.fetch()])
    }
    fn immediate(&mut self) -> u16 {
        let a = self.cpu.pc;
        self.cpu.pc = a.wrapping_add(1);
        a
    }
    fn zeropage(&mut self) -> u16 {
        self.fetch() as u16
    }
    fn zeropage_x(&mut self) -> u16 {
        self.fetch().wrapping_add(self.cpu.x) as u16
    }
    fn zeropage_y(&mut self) -> u16 {
        self.fetch().wrapping_add(self.cpu.y) as u16
    }
    fn absolute(&mut self) -> u16 {
        self.word()
    }
    fn absolute_x(&mut self) -> u16 {
        self.word().wrapping_add(self.cpu.x as u16)
    }
    fn absolute_y(&mut self) -> u16 {
        self.word().wrapping_add(self.cpu.y as u16)
    }
    fn indirect(&mut self) -> u16 {
        let a = self.word();
        let lo = self.read(a);
        u16::from_le_bytes([lo, self.read((a & 0xff00) | a.wrapping_add(1) & 255)])
    }
    fn indirect_x(&mut self) -> u16 {
        let a = self.fetch().wrapping_add(self.cpu.x);
        let lo = self.read(a as u16);
        u16::from_le_bytes([lo, self.read(a.wrapping_add(1) as u16)])
    }
    fn indirect_y(&mut self) -> u16 {
        let a = self.fetch();
        let lo = self.read(a as u16);
        u16::from_le_bytes([lo, self.read(a.wrapping_add(1) as u16)])
            .wrapping_add(self.cpu.y as u16)
    }
    pub(super) fn interrupt(&mut self, addr: u16, nmi: bool) {
        self.push((self.cpu.pc >> 8) as u8);
        self.push(self.cpu.pc as u8);
        self.push(self.cpu.p & !0x10 | 0x20);
        self.cpu.pc = addr;
        if !nmi {
            self.cpu.p |= I;
        }
        self.interrupts += 1;
        self.halted = false;
    }
    pub fn step_cpu(&mut self) -> Result<u32, String> {
        let pending = u16::from_be_bytes([self.reg[0x73], self.reg[0x74]]);
        let prospective = if pending != 0 && self.cpu.p & I == 0 {
            self.vector(pending.leading_zeros() as usize)
        } else {
            self.cpu.pc
        };
        let opcode = if (0x3000..=0x3fff).contains(&prospective) {
            return Err(format!(
                "Instruction fetch from Tama-Go peripheral space at {prospective:04X}"
            ));
        } else {
            self.read(prospective)
        };
        let cycles = CYCLES[opcode as usize];
        if cycles == 0 {
            return Err(format!(
                "Unsupported Tama-Go opcode {opcode:02X} at {prospective:04X}"
            ));
        }
        if prospective != self.cpu.pc || (pending != 0 && self.cpu.p & I == 0) {
            self.interrupt(prospective, false);
        }
        self.fetch();
        match opcode {
            0xA9 => {
                let a = self.immediate();
                self.operand("LDA", a);
            }
            0xA5 => {
                let a = self.zeropage();
                self.operand("LDA", a);
            }
            0xB5 => {
                let a = self.zeropage_x();
                self.operand("LDA", a);
            }
            0xAD => {
                let a = self.absolute();
                self.operand("LDA", a);
            }
            0xBD => {
                let a = self.absolute_x();
                self.operand("LDA", a);
            }
            0xB9 => {
                let a = self.absolute_y();
                self.operand("LDA", a);
            }
            0xA1 => {
                let a = self.indirect_x();
                self.operand("LDA", a);
            }
            0xB1 => {
                let a = self.indirect_y();
                self.operand("LDA", a);
            }
            0xA2 => {
                let a = self.immediate();
                self.operand("LDX", a);
            }
            0xA6 => {
                let a = self.zeropage();
                self.operand("LDX", a);
            }
            0xB6 => {
                let a = self.zeropage_y();
                self.operand("LDX", a);
            }
            0xAE => {
                let a = self.absolute();
                self.operand("LDX", a);
            }
            0xBE => {
                let a = self.absolute_y();
                self.operand("LDX", a);
            }
            0xA0 => {
                let a = self.immediate();
                self.operand("LDY", a);
            }
            0xA4 => {
                let a = self.zeropage();
                self.operand("LDY", a);
            }
            0xB4 => {
                let a = self.zeropage_x();
                self.operand("LDY", a);
            }
            0xAC => {
                let a = self.absolute();
                self.operand("LDY", a);
            }
            0xBC => {
                let a = self.absolute_x();
                self.operand("LDY", a);
            }
            0x85 => {
                let a = self.zeropage();
                self.operand("STA", a);
            }
            0x95 => {
                let a = self.zeropage_x();
                self.operand("STA", a);
            }
            0x8D => {
                let a = self.absolute();
                self.operand("STA", a);
            }
            0x9D => {
                let a = self.absolute_x();
                self.operand("STA", a);
            }
            0x99 => {
                let a = self.absolute_y();
                self.operand("STA", a);
            }
            0x81 => {
                let a = self.indirect_x();
                self.operand("STA", a);
            }
            0x91 => {
                let a = self.indirect_y();
                self.operand("STA", a);
            }
            0x86 => {
                let a = self.zeropage();
                self.operand("STX", a);
            }
            0x96 => {
                let a = self.zeropage_y();
                self.operand("STX", a);
            }
            0x8E => {
                let a = self.absolute();
                self.operand("STX", a);
            }
            0x84 => {
                let a = self.zeropage();
                self.operand("STY", a);
            }
            0x94 => {
                let a = self.zeropage_x();
                self.operand("STY", a);
            }
            0x8C => {
                let a = self.absolute();
                self.operand("STY", a);
            }
            0xAA => {
                self.implied("TAX");
            }
            0xA8 => {
                self.implied("TAY");
            }
            0x8A => {
                self.implied("TXA");
            }
            0x98 => {
                self.implied("TYA");
            }
            0xBA => {
                self.implied("TSX");
            }
            0x9A => {
                self.implied("TXS");
            }
            0xE8 => {
                self.implied("INX");
            }
            0xC8 => {
                self.implied("INY");
            }
            0xCA => {
                self.implied("DEX");
            }
            0x88 => {
                self.implied("DEY");
            }
            0xE6 => {
                let a = self.zeropage();
                self.operand("INC", a);
            }
            0xF6 => {
                let a = self.zeropage_x();
                self.operand("INC", a);
            }
            0xEE => {
                let a = self.absolute();
                self.operand("INC", a);
            }
            0xFE => {
                let a = self.absolute_x();
                self.operand("INC", a);
            }
            0xC6 => {
                let a = self.zeropage();
                self.operand("DEC", a);
            }
            0xD6 => {
                let a = self.zeropage_x();
                self.operand("DEC", a);
            }
            0xCE => {
                let a = self.absolute();
                self.operand("DEC", a);
            }
            0xDE => {
                let a = self.absolute_x();
                self.operand("DEC", a);
            }
            0x29 => {
                let a = self.immediate();
                self.operand("AND", a);
            }
            0x25 => {
                let a = self.zeropage();
                self.operand("AND", a);
            }
            0x35 => {
                let a = self.zeropage_x();
                self.operand("AND", a);
            }
            0x2D => {
                let a = self.absolute();
                self.operand("AND", a);
            }
            0x3D => {
                let a = self.absolute_x();
                self.operand("AND", a);
            }
            0x39 => {
                let a = self.absolute_y();
                self.operand("AND", a);
            }
            0x21 => {
                let a = self.indirect_x();
                self.operand("AND", a);
            }
            0x31 => {
                let a = self.indirect_y();
                self.operand("AND", a);
            }
            0x09 => {
                let a = self.immediate();
                self.operand("ORA", a);
            }
            0x05 => {
                let a = self.zeropage();
                self.operand("ORA", a);
            }
            0x15 => {
                let a = self.zeropage_x();
                self.operand("ORA", a);
            }
            0x0D => {
                let a = self.absolute();
                self.operand("ORA", a);
            }
            0x1D => {
                let a = self.absolute_x();
                self.operand("ORA", a);
            }
            0x19 => {
                let a = self.absolute_y();
                self.operand("ORA", a);
            }
            0x01 => {
                let a = self.indirect_x();
                self.operand("ORA", a);
            }
            0x11 => {
                let a = self.indirect_y();
                self.operand("ORA", a);
            }
            0x49 => {
                let a = self.immediate();
                self.operand("EOR", a);
            }
            0x45 => {
                let a = self.zeropage();
                self.operand("EOR", a);
            }
            0x55 => {
                let a = self.zeropage_x();
                self.operand("EOR", a);
            }
            0x4D => {
                let a = self.absolute();
                self.operand("EOR", a);
            }
            0x5D => {
                let a = self.absolute_x();
                self.operand("EOR", a);
            }
            0x59 => {
                let a = self.absolute_y();
                self.operand("EOR", a);
            }
            0x41 => {
                let a = self.indirect_x();
                self.operand("EOR", a);
            }
            0x51 => {
                let a = self.indirect_y();
                self.operand("EOR", a);
            }
            0x24 => {
                let a = self.zeropage();
                self.operand("BIT", a);
            }
            0x2C => {
                let a = self.absolute();
                self.operand("BIT", a);
            }
            0x0A => {
                self.shift("ASL", None);
            }
            0x06 => {
                let a = self.zeropage();
                self.shift("ASL", Some(a));
            }
            0x16 => {
                let a = self.zeropage_x();
                self.shift("ASL", Some(a));
            }
            0x0E => {
                let a = self.absolute();
                self.shift("ASL", Some(a));
            }
            0x1E => {
                let a = self.absolute_x();
                self.shift("ASL", Some(a));
            }
            0x4A => {
                self.shift("LSR", None);
            }
            0x46 => {
                let a = self.zeropage();
                self.shift("LSR", Some(a));
            }
            0x56 => {
                let a = self.zeropage_x();
                self.shift("LSR", Some(a));
            }
            0x4E => {
                let a = self.absolute();
                self.shift("LSR", Some(a));
            }
            0x5E => {
                let a = self.absolute_x();
                self.shift("LSR", Some(a));
            }
            0x2A => {
                self.shift("ROL", None);
            }
            0x26 => {
                let a = self.zeropage();
                self.shift("ROL", Some(a));
            }
            0x36 => {
                let a = self.zeropage_x();
                self.shift("ROL", Some(a));
            }
            0x2E => {
                let a = self.absolute();
                self.shift("ROL", Some(a));
            }
            0x3E => {
                let a = self.absolute_x();
                self.shift("ROL", Some(a));
            }
            0x6A => {
                self.shift("ROR", None);
            }
            0x66 => {
                let a = self.zeropage();
                self.shift("ROR", Some(a));
            }
            0x76 => {
                let a = self.zeropage_x();
                self.shift("ROR", Some(a));
            }
            0x6E => {
                let a = self.absolute();
                self.shift("ROR", Some(a));
            }
            0x7E => {
                let a = self.absolute_x();
                self.shift("ROR", Some(a));
            }
            0x69 => {
                let a = self.immediate();
                self.operand("ADC", a);
            }
            0x65 => {
                let a = self.zeropage();
                self.operand("ADC", a);
            }
            0x75 => {
                let a = self.zeropage_x();
                self.operand("ADC", a);
            }
            0x6D => {
                let a = self.absolute();
                self.operand("ADC", a);
            }
            0x7D => {
                let a = self.absolute_x();
                self.operand("ADC", a);
            }
            0x79 => {
                let a = self.absolute_y();
                self.operand("ADC", a);
            }
            0x61 => {
                let a = self.indirect_x();
                self.operand("ADC", a);
            }
            0x71 => {
                let a = self.indirect_y();
                self.operand("ADC", a);
            }
            0xE9 => {
                let a = self.immediate();
                self.operand("SBC", a);
            }
            0xE5 => {
                let a = self.zeropage();
                self.operand("SBC", a);
            }
            0xF5 => {
                let a = self.zeropage_x();
                self.operand("SBC", a);
            }
            0xED => {
                let a = self.absolute();
                self.operand("SBC", a);
            }
            0xFD => {
                let a = self.absolute_x();
                self.operand("SBC", a);
            }
            0xF9 => {
                let a = self.absolute_y();
                self.operand("SBC", a);
            }
            0xE1 => {
                let a = self.indirect_x();
                self.operand("SBC", a);
            }
            0xF1 => {
                let a = self.indirect_y();
                self.operand("SBC", a);
            }
            0xC9 => {
                let a = self.immediate();
                self.operand("CMP", a);
            }
            0xC5 => {
                let a = self.zeropage();
                self.operand("CMP", a);
            }
            0xD5 => {
                let a = self.zeropage_x();
                self.operand("CMP", a);
            }
            0xCD => {
                let a = self.absolute();
                self.operand("CMP", a);
            }
            0xDD => {
                let a = self.absolute_x();
                self.operand("CMP", a);
            }
            0xD9 => {
                let a = self.absolute_y();
                self.operand("CMP", a);
            }
            0xC1 => {
                let a = self.indirect_x();
                self.operand("CMP", a);
            }
            0xD1 => {
                let a = self.indirect_y();
                self.operand("CMP", a);
            }
            0xE0 => {
                let a = self.immediate();
                self.operand("CPX", a);
            }
            0xE4 => {
                let a = self.zeropage();
                self.operand("CPX", a);
            }
            0xEC => {
                let a = self.absolute();
                self.operand("CPX", a);
            }
            0xC0 => {
                let a = self.immediate();
                self.operand("CPY", a);
            }
            0xC4 => {
                let a = self.zeropage();
                self.operand("CPY", a);
            }
            0xCC => {
                let a = self.absolute();
                self.operand("CPY", a);
            }
            0x18 => {
                self.implied("CLC");
            }
            0x38 => {
                self.implied("SEC");
            }
            0x58 => {
                self.implied("CLI");
            }
            0x78 => {
                self.implied("SEI");
            }
            0xB8 => {
                self.implied("CLV");
            }
            0xD8 => {
                self.implied("CLD");
            }
            0xF8 => {
                self.implied("SED");
            }
            0x48 => {
                self.implied("PHA");
            }
            0x68 => {
                self.implied("PLA");
            }
            0x08 => {
                self.implied("PHP");
            }
            0x28 => {
                self.implied("PLP");
            }
            0x4C => {
                let a = self.absolute();
                self.operand("JMP", a);
            }
            0x6C => {
                let a = self.indirect();
                self.operand("JMP", a);
            }
            0x20 => {
                let a = self.absolute();
                self.operand("JSR", a);
            }
            0x60 => {
                self.implied("RTS");
            }
            0x40 => {
                self.implied("RTI");
            }
            0x00 => {
                self.implied("BRK");
            }
            0x10 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & N != 0) == false {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0x30 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & N != 0) == true {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0x50 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & V != 0) == false {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0x70 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & V != 0) == true {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0x90 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & C != 0) == false {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0xB0 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & C != 0) == true {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0xD0 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & Z != 0) == false {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0xF0 => {
                let d = self.fetch() as i8;
                if (self.cpu.p & Z != 0) == true {
                    self.cpu.pc = self.cpu.pc.wrapping_add_signed(d as i16);
                }
            }
            0xEA => {}
            _ => unreachable!(),
        }
        self.instructions += 1;
        Ok(cycles as u32)
    }
    fn operand(&mut self, op: &str, addr: u16) {
        match op {
            "STA" => self.write(addr, self.cpu.a),
            "STX" => self.write(addr, self.cpu.x),
            "STY" => self.write(addr, self.cpu.y),
            "JMP" => self.cpu.pc = addr,
            "JSR" => {
                let r = self.cpu.pc.wrapping_sub(1);
                self.push((r >> 8) as u8);
                self.push(r as u8);
                self.cpu.pc = addr;
            }
            _ => {
                let d = self.read(addr);
                match op {
                    "LDA" => {
                        self.cpu.a = d;
                        self.nz(d);
                    }
                    "LDX" => {
                        self.cpu.x = d;
                        self.nz(d);
                    }
                    "LDY" => {
                        self.cpu.y = d;
                        self.nz(d);
                    }
                    "INC" | "DEC" => {
                        let v = if op == "INC" {
                            d.wrapping_add(1)
                        } else {
                            d.wrapping_sub(1)
                        };
                        self.nz(v);
                        self.write(addr, v);
                    }
                    "AND" | "ORA" | "EOR" => {
                        self.cpu.a = match op {
                            "AND" => self.cpu.a & d,
                            "ORA" => self.cpu.a | d,
                            _ => self.cpu.a ^ d,
                        };
                        self.nz(self.cpu.a);
                    }
                    "BIT" => {
                        self.flag(V, d & 64 != 0);
                        self.flag(N, d & 128 != 0);
                        self.flag(Z, self.cpu.a & d == 0);
                    }
                    "CMP" | "CPX" | "CPY" => {
                        let v = match op {
                            "CMP" => self.cpu.a,
                            "CPX" => self.cpu.x,
                            _ => self.cpu.y,
                        };
                        self.flag(C, v >= d);
                        self.nz(v.wrapping_sub(d));
                    }
                    "ADC" => {
                        let a = self.cpu.a;
                        let c = i32::from(self.cpu.p & C != 0);
                        let mut o = a as i32 + d as i32 + c;
                        self.flag(V, (!(a ^ d) & (a ^ (o as u8))) & 128 != 0);
                        self.nz(o as u8);
                        if self.cpu.p & D != 0 {
                            let mut al = (a & 15) as i32 + (d & 15) as i32 + c;
                            let mut ah =
                                (a & 240) as i32 + (d & 240) as i32 + if al >= 16 { 16 } else { 0 };
                            if al > 9 {
                                al += 6;
                            }
                            if ah > 0x90 {
                                ah += 0x60;
                            }
                            o = (al & 15) + ah;
                        }
                        self.flag(C, o > 255);
                        self.cpu.a = o as u8;
                    }
                    "SBC" => {
                        let a = self.cpu.a;
                        let mut o = a as i32 - d as i32 - i32::from(self.cpu.p & C == 0);
                        self.flag(V, ((a ^ d) & (a ^ (o as u8))) & 128 != 0);
                        self.nz(o as u8);
                        self.flag(C, o >= 0);
                        // Preserve the reference's post-subtraction carry in decimal adjustment.
                        if self.cpu.p & D != 0 {
                            let mut al =
                                (a & 15) as i32 - (d & 15) as i32 - i32::from(self.cpu.p & C == 0);
                            let mut ah =
                                (a & 240) as i32 - (d & 240) as i32 - if al < 0 { 16 } else { 0 };
                            if al < 0 {
                                al -= 6;
                            }
                            if ah < 0 {
                                ah -= 0x60;
                            }
                            o = (al & 15) + ah;
                        }
                        self.cpu.a = o as u8;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    fn shift(&mut self, op: &str, addr: Option<u16>) {
        let d = addr.map(|a| self.read(a)).unwrap_or(self.cpu.a);
        let c = self.cpu.p & C != 0;
        let v = match op {
            "ASL" => d << 1,
            "LSR" => d >> 1,
            "ROL" => (d << 1) | u8::from(c),
            _ => (d >> 1) | if c { 128 } else { 0 },
        };
        self.flag(
            C,
            if op == "ASL" || op == "ROL" {
                d & 128 != 0
            } else {
                d & 1 != 0
            },
        );
        self.nz(v);
        if let Some(a) = addr {
            self.write(a, v)
        } else {
            self.cpu.a = v;
        }
    }
    fn implied(&mut self, op: &str) {
        match op {
            "TAX" => {
                self.cpu.x = self.cpu.a;
                self.nz(self.cpu.x)
            }
            "TAY" => {
                self.cpu.y = self.cpu.a;
                self.nz(self.cpu.y)
            }
            "TXA" => {
                self.cpu.a = self.cpu.x;
                self.nz(self.cpu.a)
            }
            "TYA" => {
                self.cpu.a = self.cpu.y;
                self.nz(self.cpu.a)
            }
            "TSX" => {
                self.cpu.x = self.cpu.s;
                self.nz(self.cpu.x)
            }
            "TXS" => self.cpu.s = self.cpu.x,
            "INX" => {
                self.cpu.x = self.cpu.x.wrapping_add(1);
                self.nz(self.cpu.x)
            }
            "INY" => {
                self.cpu.y = self.cpu.y.wrapping_add(1);
                self.nz(self.cpu.y)
            }
            "DEX" => {
                self.cpu.x = self.cpu.x.wrapping_sub(1);
                self.nz(self.cpu.x)
            }
            "DEY" => {
                self.cpu.y = self.cpu.y.wrapping_sub(1);
                self.nz(self.cpu.y)
            }
            "CLC" => self.flag(C, false),
            "SEC" => self.flag(C, true),
            "CLI" => self.flag(I, false),
            "SEI" => self.flag(I, true),
            "CLV" => self.flag(V, false),
            "CLD" => self.flag(D, false),
            "SED" => self.flag(D, true),
            "PHA" => self.push(self.cpu.a),
            "PHP" => self.push(self.cpu.p | 0x30),
            "PLA" => {
                self.cpu.a = self.pull();
                self.nz(self.cpu.a)
            }
            "PLP" => self.cpu.p = self.pull() & !0x10 | 0x20,
            "RTS" => {
                let lo = self.pull();
                let hi = self.pull();
                self.cpu.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
            }
            "RTI" => {
                self.cpu.p = self.pull() & !0x10 | 0x20;
                let lo = self.pull();
                let hi = self.pull();
                self.cpu.pc = u16::from_le_bytes([lo, hi]);
            }
            "BRK" => {
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.push((self.cpu.pc >> 8) as u8);
                self.push(self.cpu.pc as u8);
                self.push(self.cpu.p | 0x30);
                self.cpu.pc = self.read16(0xfffe);
                self.flag(I, true);
            }
            _ => unreachable!(),
        }
    }
}
const CYCLES: [u8; 256] = [
    7, 6, 0, 0, 0, 3, 5, 0, 3, 2, 2, 0, 0, 4, 6, 0, 2, 5, 0, 0, 0, 4, 6, 0, 2, 4, 0, 0, 0, 4, 7, 0,
    6, 6, 0, 0, 3, 3, 5, 0, 4, 2, 2, 0, 4, 4, 6, 0, 2, 5, 0, 0, 0, 4, 6, 0, 2, 4, 0, 0, 0, 4, 7, 0,
    6, 6, 0, 0, 0, 3, 5, 0, 3, 2, 2, 0, 3, 4, 6, 0, 2, 5, 0, 0, 0, 4, 6, 0, 2, 4, 0, 0, 0, 4, 7, 0,
    6, 6, 0, 0, 0, 3, 5, 0, 4, 2, 2, 0, 5, 4, 6, 0, 2, 5, 0, 0, 0, 4, 6, 0, 2, 4, 0, 0, 0, 4, 7, 0,
    0, 6, 0, 0, 3, 3, 3, 0, 2, 0, 2, 0, 4, 4, 4, 0, 2, 6, 0, 0, 4, 4, 4, 0, 2, 5, 2, 0, 0, 5, 0, 0,
    2, 6, 2, 0, 3, 3, 3, 0, 2, 2, 2, 0, 4, 4, 4, 0, 2, 5, 0, 0, 4, 4, 4, 0, 2, 4, 2, 0, 4, 4, 4, 0,
    2, 6, 0, 0, 3, 3, 5, 0, 2, 2, 2, 0, 4, 4, 6, 0, 2, 5, 0, 0, 0, 4, 6, 0, 2, 4, 0, 0, 0, 4, 7, 0,
    2, 6, 0, 0, 3, 3, 5, 0, 2, 2, 2, 0, 4, 4, 6, 0, 2, 5, 0, 0, 0, 4, 6, 0, 2, 4, 0, 0, 0, 4, 7, 0,
];
