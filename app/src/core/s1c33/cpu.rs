// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros (Rust translation and integration)
// Derived from maragotchi/tamaemu/src/cpu.c, revision 79e19b65a1c970b9e1cd0b58ee6e929507257234.
// The original credits GMMan/s1c33_sleigh as its S1C33 PE instruction-set reference.
//! Native S1C33 interpreter. Peripheral scheduling belongs above this CPU.
use super::memory::{Bus, BusError};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
    pub ie: bool,
    pub level: u8,
    pub rest: u32,
}
impl Flags {
    pub fn pack(self) -> u32 {
        (self.rest & !0xf1f)
            | (self.n as u32)
            | ((self.z as u32) << 1)
            | ((self.v as u32) << 2)
            | ((self.c as u32) << 3)
            | ((self.ie as u32) << 4)
            | (((self.level & 15) as u32) << 8)
    }
    pub fn unpack(&mut self, value: u32) {
        *self = Self {
            n: value & 1 != 0,
            z: value & 2 != 0,
            v: value & 4 != 0,
            c: value & 8 != 0,
            ie: value & 16 != 0,
            level: ((value >> 8) & 15) as u8,
            rest: value,
        };
    }
    fn result(&mut self, value: u32) {
        self.n = value >> 31 != 0;
        self.z = value == 0;
    }
    fn logic(&mut self, value: u32) -> u32 {
        self.result(value);
        self.v = false;
        value
    }
    fn add(&mut self, a: u32, b: u32) -> u32 {
        let (r, c) = a.overflowing_add(b);
        self.v = overflow_add(a, b);
        self.c = c;
        self.result(r);
        r
    }
    fn adc(&mut self, a: u32, b: u32) -> u32 {
        let carry = self.c as u32;
        let (s, c1) = a.overflowing_add(b);
        let (r, c2) = s.overflowing_add(carry);
        self.v = overflow_add(a, b) ^ overflow_add(s, carry);
        self.c = c1 || c2;
        self.result(r);
        r
    }
    fn sub(&mut self, a: u32, b: u32) -> u32 {
        let (r, c) = a.overflowing_sub(b);
        self.v = overflow_sub(a, b);
        self.c = c;
        self.result(r);
        r
    }
    fn sbc(&mut self, a: u32, b: u32) -> u32 {
        let carry = self.c as u32;
        let (d, c1) = a.overflowing_sub(b);
        let (r, c2) = d.overflowing_sub(carry);
        self.v = overflow_sub(a, b) || overflow_sub(d, carry);
        self.c = c1 || c2;
        self.result(r);
        r
    }
}
fn overflow_add(a: u32, b: u32) -> bool {
    (!(a ^ b) & (a ^ a.wrapping_add(b))) >> 31 != 0
}
fn overflow_sub(a: u32, b: u32) -> bool {
    ((a ^ b) & (a ^ a.wrapping_sub(b))) >> 31 != 0
}
fn sext(value: u32, bits: u32) -> u32 {
    let sign = 1 << (bits - 1);
    (value ^ sign).wrapping_sub(sign)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cpu {
    pub r: [u32; 16],
    pub sp: u32,
    pub alr: u32,
    pub ahr: u32,
    pub ttbr: u32,
    pub idir: u32,
    pub dbbr: u32,
    pub pc: u32,
    pub flags: Flags,
    pub ext_count: u8,
    pub ext_values: [u32; 2],
    pub halted: bool,
    pub hard_halt: bool,
    pub wake_at: u64,
    pub cycles: u64,
    pub halt_wakes: u64,
    /// Interrupt-controller lines, sampled only at instruction boundaries.
    pub pending_irq: u32,
    pub pending_level: u8,
    pub wake_request: bool,
    /// CMU clock-option latch: determines whether SLP waits for oscillator stabilization.
    pub clock_option: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    FetchUnmapped(u32),
    TooManyPrefixes,
    InvalidEncoding(u16),
    InvalidSpecialRegister(u8),
    Break,
    DebugReturn,
    CoprocessorUnavailable,
    UnsupportedPsrBit(u8),
    DelayDepthExceeded,
    Bus(BusError),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuError {
    pub pc: u32,
    pub fault: Fault,
}
impl std::fmt::Display for CpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S1C33 error at {:#010x}: {:?}", self.pc, self.fault)
    }
}
impl std::error::Error for CpuError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    BranchInDelaySlot(u32),
    PrefixInDelaySlot(u32),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub cycles: u64,
    pub instruction: bool,
    pub interrupt: Option<u32>,
    pub notices: Vec<Notice>,
}

impl Cpu {
    /// Reset CPU registers as in cpu_reset; elapsed cycles and external lines are retained.
    pub fn reset(&mut self, bus: &mut impl Bus, ttbr: u32) -> Result<(), CpuError> {
        let pc = bus.read32(ttbr).map_err(|e| CpuError {
            pc: self.pc,
            fault: Fault::Bus(e),
        })?;
        self.r = [0; 16];
        self.sp = 0;
        self.alr = 0;
        self.ahr = 0;
        self.ttbr = ttbr;
        self.idir = 0;
        self.dbbr = 0;
        self.pc = pc;
        self.flags = Flags::default();
        self.ext_count = 0;
        self.halted = false;
        self.hard_halt = false;
        Ok(())
    }
    /// Failed instructions roll back both CPU and bus side effects, including delay slots.
    pub fn step(&mut self, bus: &mut impl Bus) -> Result<Step, CpuError> {
        let before = self.clone();
        bus.begin();
        let mut outcome = Step::default();
        let result = self.accept_and_step(bus, &mut outcome);
        match result {
            Ok(()) => {
                outcome.cycles = self.cycles.wrapping_sub(before.cycles);
                bus.commit();
                Ok(outcome)
            }
            Err(fault) => {
                let error = CpuError { pc: self.pc, fault };
                *self = before;
                bus.rollback();
                Err(error)
            }
        }
    }
    fn read8(&self, bus: &mut impl Bus, a: u32) -> Result<u8, Fault> {
        bus.read8(a).map_err(Fault::Bus)
    }
    fn read16(&self, bus: &mut impl Bus, a: u32) -> Result<u16, Fault> {
        bus.read16(a).map_err(Fault::Bus)
    }
    fn read32(&self, bus: &mut impl Bus, a: u32) -> Result<u32, Fault> {
        bus.read32(a).map_err(Fault::Bus)
    }
    fn push(&mut self, bus: &mut impl Bus, value: u32) -> Result<(), Fault> {
        self.sp = self.sp.wrapping_sub(4);
        bus.write32(self.sp, value).map_err(Fault::Bus)
    }
    fn pop(&mut self, bus: &mut impl Bus) -> Result<u32, Fault> {
        let value = self.read32(bus, self.sp)?;
        self.sp = self.sp.wrapping_add(4);
        Ok(value)
    }
    fn accept_and_step(&mut self, bus: &mut impl Bus, out: &mut Step) -> Result<(), Fault> {
        if self.halted {
            self.cycles = self.cycles.wrapping_add(16);
            if self.pending_irq != 0
                || self.wake_request
                || (!self.hard_halt && self.cycles >= self.wake_at)
            {
                self.halted = false;
                self.halt_wakes = self.halt_wakes.wrapping_add(1);
            } else {
                return Ok(());
            }
        }
        if self.pending_irq != 0 && self.flags.ie && self.pending_level > self.flags.level {
            let vector = self.pending_irq;
            self.push(bus, self.pc)?;
            self.push(bus, self.flags.pack())?;
            self.flags.ie = false;
            self.flags.level = self.pending_level;
            self.pc = self.read32(bus, self.ttbr.wrapping_add(vector.wrapping_mul(4)))?;
            self.pending_irq = 0;
            out.interrupt = Some(vector);
        }
        self.step_one(bus, false, 0, out)?;
        out.instruction = true;
        Ok(())
    }
    fn step_one(
        &mut self,
        bus: &mut impl Bus,
        in_slot: bool,
        depth: u8,
        out: &mut Step,
    ) -> Result<(), Fault> {
        if depth >= 64 {
            return Err(Fault::DelayDepthExceeded);
        }
        self.pc &= 0x0fff_ffff;
        if !bus.executable(self.pc) {
            return Err(Fault::FetchUnmapped(self.pc));
        }
        let mut word = self.read16(bus, self.pc)?;
        let mut guard = 0;
        while word >> 13 == 6 {
            if in_slot && guard == 0 {
                out.notices.push(Notice::PrefixInDelaySlot(self.pc));
            }
            if self.ext_count < 2 {
                self.ext_values[self.ext_count as usize] = (word & 0x1fff) as u32;
            }
            self.ext_count = self.ext_count.saturating_add(1);
            self.pc = self.pc.wrapping_add(2);
            self.cycles = self.cycles.wrapping_add(1);
            word = self.read16(bus, self.pc)?;
            guard += 1;
            if guard > 2 {
                return Err(Fault::TooManyPrefixes);
            }
        }
        let address = self.pc;
        self.pc = self.pc.wrapping_add(2);
        self.cycles = self.cycles.wrapping_add(1);
        bus.cpu_cycle(self.cycles);
        self.execute(bus, word, address, in_slot, depth, out)
    }
    fn special_read(&self, index: u8, address: u32) -> Result<u32, Fault> {
        Ok(match index {
            0 => self.flags.pack(),
            1 => self.sp,
            2 => self.alr,
            3 => self.ahr,
            8 => self.ttbr,
            10 => self.idir,
            11 => self.dbbr,
            15 => address,
            _ => return Err(Fault::InvalidSpecialRegister(index)),
        })
    }
    fn special_write(&mut self, index: u8, value: u32) -> Result<(), Fault> {
        match index {
            0 => self.flags.unpack(value),
            1 => self.sp = value,
            2 => self.alr = value,
            3 => self.ahr = value,
            8 => self.ttbr = value,
            10 => self.idir = value,
            11 => self.dbbr = value,
            15 => self.pc = value,
            _ => return Err(Fault::InvalidSpecialRegister(index)),
        }
        Ok(())
    }
    fn condition(&self, op: u16) -> bool {
        let f = self.flags;
        match op {
            4 => !f.z && f.n == f.v,
            5 => f.n == f.v,
            6 => f.n != f.v,
            7 => f.z || f.n != f.v,
            8 => !f.z && !f.c,
            9 => !f.c,
            10 => f.c,
            11 => f.z || f.c,
            12 => f.z,
            13 => !f.z,
            _ => true,
        }
    }
    fn shift(&mut self, value: u32, amount: u32, kind: u16) -> u32 {
        let n = amount & 31;
        let result = match kind {
            0 => value >> n,
            1 | 3 => value << n,
            2 => ((value as i32) >> n) as u32,
            4 => value.rotate_right(n),
            5 => value.rotate_left(n),
            _ => unreachable!(),
        };
        self.flags.result(result);
        result
    }
    fn transfer(
        &mut self,
        bus: &mut impl Bus,
        op: u16,
        rd: usize,
        address: u32,
    ) -> Result<u32, Fault> {
        match op {
            0 => self.r[rd] = sext(self.read8(bus, address)? as u32, 8),
            1 => self.r[rd] = self.read8(bus, address)? as u32,
            2 => self.r[rd] = sext(self.read16(bus, address)? as u32, 16),
            3 => self.r[rd] = self.read16(bus, address)? as u32,
            4 => self.r[rd] = self.read32(bus, address)?,
            5 => bus.write8(address, self.r[rd] as u8).map_err(Fault::Bus)?,
            6 => bus
                .write16(address, self.r[rd] as u16)
                .map_err(Fault::Bus)?,
            7 => bus.write32(address, self.r[rd]).map_err(Fault::Bus)?,
            _ => unreachable!(),
        }
        Ok([1, 1, 2, 2, 4, 1, 2, 4][op as usize])
    }
    fn alu(&mut self, op: u16, a: u32, b: u32) -> u32 {
        match op {
            0 => self.flags.add(a, b),
            1 | 2 => self.flags.sub(a, b),
            3 => b,
            4 => self.flags.logic(a & b),
            5 => self.flags.logic(a | b),
            6 => self.flags.logic(a ^ b),
            7 => self.flags.logic(!b),
            _ => unreachable!(),
        }
    }
    fn execute(
        &mut self,
        bus: &mut impl Bus,
        word: u16,
        address: u32,
        in_slot: bool,
        depth: u8,
        out: &mut Step,
    ) -> Result<(), Fault> {
        let rd = (word & 15) as usize;
        let rs = ((word >> 4) & 15) as usize;
        let ext_n = self.ext_count.min(2);
        let [ext0, ext1] = self.ext_values;
        self.ext_count = 0;
        let ext = if ext_n == 1 {
            ext0
        } else {
            (ext0 << 13) | ext1
        };
        let op1 = (word >> 10) & 7;
        let op2 = (word >> 8) & 3;
        match word >> 13 {
            0 => {
                let op = (word >> 9) & 15;
                let delayed = word & 0x100 != 0;
                if op <= 3 {
                    let constant = (word >> 4) & 3;
                    if constant == 0 {
                        let group = op * 8 + (delayed as u16) * 4 + ((word >> 6) & 3);
                        match group {
                            0 => {}
                            1 => {
                                self.halted = true;
                                self.hard_halt = self.clock_option & 0x3e00 == 0;
                                self.wake_at = self.cycles.wrapping_add(20000);
                            }
                            2 => {
                                self.halted = true;
                                self.hard_halt = true;
                            }
                            8 => {
                                for i in (0..=rd).rev() {
                                    self.push(bus, self.r[i])?;
                                }
                            }
                            9 => {
                                for i in 0..=rd {
                                    self.r[i] = self.pop(bus)?;
                                }
                            }
                            11 => self.pc = address.wrapping_add(self.r[rd]),
                            15 => {
                                let target = address.wrapping_add(self.r[rd]);
                                self.step_one(bus, true, depth + 1, out)?;
                                self.pc = target;
                            }
                            16 => return Err(Fault::Break),
                            17 => return Err(Fault::DebugReturn),
                            18 => {
                                let vector = 12 + (word & 3) as u32;
                                self.push(bus, self.pc)?;
                                self.push(bus, self.flags.pack())?;
                                self.flags.ie = false;
                                self.pc = self.read32(bus, self.ttbr.wrapping_add(vector * 4))?;
                            }
                            19 => {
                                let flags = self.pop(bus)?;
                                self.flags.unpack(flags);
                                self.pc = self.pop(bus)?;
                            }
                            24 => {
                                self.push(bus, self.pc)?;
                                self.pc = self.r[rd];
                            }
                            25 => self.pc = self.pop(bus)?,
                            26 => self.pc = self.r[rd],
                            28 => {
                                let target = self.r[rd];
                                self.step_one(bus, true, depth + 1, out)?;
                                self.push(bus, self.pc)?;
                                self.pc = target;
                            }
                            29 => {
                                self.step_one(bus, true, depth + 1, out)?;
                                self.pc = self.pop(bus)?;
                            }
                            30 => {
                                let target = self.r[rd];
                                self.step_one(bus, true, depth + 1, out)?;
                                self.pc = target;
                            }
                            _ => return Err(Fault::InvalidEncoding(word)),
                        }
                    } else if constant == 1 && op == 0 {
                        match (word >> 6) & 7 {
                            0 => self.push(bus, self.r[rd])?,
                            1 => self.r[rd] = self.pop(bus)?,
                            2 => match rd {
                                2 => self.push(bus, self.alr)?,
                                3 => {
                                    self.push(bus, self.ahr)?;
                                    self.push(bus, self.alr)?;
                                }
                                _ => return Err(Fault::InvalidEncoding(word)),
                            },
                            3 => match rd {
                                2 => self.alr = self.pop(bus)?,
                                3 => {
                                    self.alr = self.pop(bus)?;
                                    self.ahr = self.pop(bus)?;
                                }
                                _ => return Err(Fault::InvalidEncoding(word)),
                            },
                            7 => {
                                self.flags.n = false;
                                self.flags.z = false;
                                self.flags.v = false;
                                self.flags.c = false;
                            }
                            _ => return Err(Fault::InvalidEncoding(word)),
                        }
                    } else {
                        return Err(Fault::InvalidEncoding(word));
                    }
                } else {
                    let byte = (word & 255) as u32;
                    let offset = match ext_n {
                        0 => sext(byte, 8).wrapping_mul(2),
                        1 => (sext(ext0, 13) << 9) | (byte << 1),
                        _ => ((ext0 & 0x1ff8) << 19) | (ext1 << 9) | (byte << 1),
                    };
                    let target = address.wrapping_add(offset);
                    let taken = self.condition(op);
                    if in_slot {
                        out.notices.push(Notice::BranchInDelaySlot(address));
                        return Ok(());
                    }
                    if delayed {
                        self.step_one(bus, true, depth + 1, out)?;
                    }
                    if op == 14 {
                        self.push(bus, self.pc)?;
                        self.pc = target;
                    } else if op == 15 || taken {
                        self.pc = target;
                    }
                }
            }
            1 => {
                if op2 == 3 {
                    if op1 > 5 {
                        return Err(Fault::InvalidEncoding(word));
                    }
                    self.r[rd] = self.shift(self.r[rd], ((word >> 4) & 15) as u32 + 16, op1);
                } else if op2 == 2 {
                    if ext_n != 0 && (op1 == 3 || op1 == 7) {
                        return Err(Fault::InvalidEncoding(word));
                    }
                    let a = if ext_n != 0 { self.r[rs] } else { self.r[rd] };
                    let b = if ext_n != 0 { ext } else { self.r[rs] };
                    let result = self.alu(op1, a, b);
                    if op1 != 2 {
                        self.r[rd] = result;
                    }
                } else {
                    if ext_n != 0 && op2 == 1 {
                        return Err(Fault::InvalidEncoding(word));
                    }
                    let at = self.r[rs].wrapping_add(if ext_n != 0 { ext } else { 0 });
                    let size = self.transfer(bus, op1, rd, at)?;
                    if op2 == 1 {
                        self.r[rs] = self.r[rs].wrapping_add(size);
                    }
                }
            }
            2 => {
                let imm = ((word >> 4) & 63) as u32;
                let offset = match ext_n {
                    0 => imm * [1, 1, 2, 2, 4, 1, 2, 4][op1 as usize],
                    1 => (ext0 << 6) | imm,
                    _ => (ext0 << 19) | (ext1 << 6) | imm,
                };
                self.transfer(bus, op1, rd, self.sp.wrapping_add(offset))?;
            }
            3 => {
                let imm6 = ((word >> 4) & 63) as u32;
                let imm = match ext_n {
                    0 => {
                        if op1 <= 1 {
                            imm6
                        } else {
                            sext(imm6, 6)
                        }
                    }
                    1 => {
                        let imm = (ext0 << 6) | imm6;
                        if op1 <= 1 { imm } else { sext(imm, 19) }
                    }
                    _ => (ext0 << 19) | (ext1 << 6) | imm6,
                };
                let result = self.alu(op1, self.r[rd], imm);
                if op1 != 2 {
                    self.r[rd] = result;
                }
            }
            4 => {
                if op1 <= 1 {
                    let amount = (word & 0x3ff) as u32 * 4;
                    self.sp = if op1 == 0 {
                        self.sp.wrapping_add(amount)
                    } else {
                        self.sp.wrapping_sub(amount)
                    };
                } else if op1 == 4 && op2 == 2 {
                    self.r[rd] = self.r[rs].swap_bytes();
                } else if op1 == 6 && op2 == 2 {
                    let v = self.r[rs];
                    self.r[rd] = ((v & 0x00ff0000) << 8)
                        | ((v >> 8) & 0x00ff0000)
                        | ((v & 255) << 8)
                        | ((v >> 8) & 255);
                } else {
                    let amount = if op2 == 0 {
                        ((word >> 4) & 15) as u32
                    } else {
                        self.r[rs]
                    };
                    self.r[rd] = self.shift(self.r[rd], amount, op1 - 2);
                }
            }
            5 => match op1 * 4 + op2 {
                0 => self.special_write(rd as u8, self.r[rs])?,
                1 => self.r[rd] = sext(self.r[rs] & 255, 8),
                2 => self.alr = ((self.r[rd] as i16 as i32) * (self.r[rs] as i16 as i32)) as u32,
                4 => self.r[rd] = self.special_read(rs as u8, address)?,
                5 => self.r[rd] = self.r[rs] & 255,
                6 => self.alr = (self.r[rd] & 65535).wrapping_mul(self.r[rs] & 65535),
                8 | 12 | 16 | 20 => {
                    let at = self.r[rs].wrapping_add(if ext_n != 0 { ext } else { 0 });
                    let bit = 1u8 << (word & 7);
                    let value = self.read8(bus, at)?;
                    match op1 {
                        2 => self.flags.z = value & bit == 0,
                        3 => bus.write8(at, value & !bit).map_err(Fault::Bus)?,
                        4 => bus.write8(at, value | bit).map_err(Fault::Bus)?,
                        5 => bus.write8(at, value ^ bit).map_err(Fault::Bus)?,
                        _ => unreachable!(),
                    }
                }
                9 => self.r[rd] = sext(self.r[rs] & 65535, 16),
                10 => {
                    let product = (self.r[rd] as i32 as i64) * (self.r[rs] as i32 as i64);
                    self.alr = product as u32;
                    self.ahr = ((product as u64) >> 32) as u32;
                }
                13 => self.r[rd] = self.r[rs] & 65535,
                14 => {
                    let product = (self.r[rd] as u64) * (self.r[rs] as u64);
                    self.alr = product as u32;
                    self.ahr = (product >> 32) as u32;
                }
                17 | 21 => return Err(Fault::CoprocessorUnavailable),
                24 => self.r[rd] = self.flags.adc(self.r[rd], self.r[rs]),
                28 => self.r[rd] = self.flags.sbc(self.r[rd], self.r[rs]),
                31 => {
                    let op = (word >> 6) & 3;
                    let bit = (word & 31) as u8;
                    if op == 0 {
                        return Err(Fault::CoprocessorUnavailable);
                    }
                    if op != 1 && op != 2 {
                        return Err(Fault::InvalidEncoding(word));
                    }
                    let value = op == 1;
                    match bit {
                        0 => self.flags.n = value,
                        1 => self.flags.z = value,
                        2 => self.flags.v = value,
                        3 => self.flags.c = value,
                        4 => self.flags.ie = value,
                        _ => return Err(Fault::UnsupportedPsrBit(bit)),
                    }
                }
                _ => return Err(Fault::InvalidEncoding(word)),
            },
            _ => return Err(Fault::InvalidEncoding(word)),
        }
        Ok(())
    }
}
