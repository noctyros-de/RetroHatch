//! E0C6200 instruction interpreter based on BrickEmuPy (CC0).
//!
//! Source: references/BrickEmuPy-main/cores/E0C6200.py.
//! Instruction execution with optional P1 I/O; MCU timing/IRQ scheduling live in core/mcu.
//! Errors leave the state unchanged. See docs/E0C6200.md.

use crate::core::mcu::io::Io;
use std::cell::{Ref, RefCell};
use std::fmt;

pub const PROGRAM_WORDS: usize = 0x2000;
pub const RAM_NIBBLES: usize = 0x300;
pub const LCD_NIBBLES: usize = 0xA0;

/// Cloneable register state; not a complete MCU save-state structure.
/// The CPU exposes this state through an immutable reference only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuState {
    pub a: u8,
    pub b: u8,
    pub ix: u16,
    pub iy: u16,
    pub sp: u8,
    /// 13-bit word address, not a byte offset.
    pub pc: u16,
    /// Matches the full internal _NPC, not just examine()["NPC"].
    pub npc: u16,
    pub carry: bool,
    pub zero: bool,
    pub decimal: bool,
    pub interrupt_enable: bool,
    pub interrupt_delay: bool,
    pub halted: bool,
}

impl Default for CpuState {
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            ix: 0,
            iy: 0,
            sp: 0,
            pc: 0x100,
            npc: 0x100,
            carry: false,
            zero: false,
            decimal: false,
            interrupt_enable: false,
            interrupt_delay: false,
            halted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    A,
    B,
    Mx,
    My,
}

impl Operand {
    fn from_bits(bits: u16) -> Self {
        match bits & 3 {
            0 => Self::A,
            1 => Self::B,
            2 => Self::Mx,
            _ => Self::My,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AluOperation {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Or,
    Xor,
    Compare,
    Fan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexNibble {
    Xh,
    Xl,
    Yh,
    Yl,
}

impl IndexNibble {
    fn from_bits(bits: u16) -> Self {
        match bits & 3 {
            0 => Self::Xh,
            1 => Self::Xl,
            2 => Self::Yh,
            _ => Self::Yl,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    CarrySet,
    CarryClear,
    ZeroSet,
    ZeroClear,
}

/// Nibble fields used by data-transfer and stack instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisterNibble {
    Xp,
    Xh,
    Xl,
    Yp,
    Yh,
    Yl,
    Sph,
    Spl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackValue {
    Operand(Operand),
    Register(RegisterNibble),
    Flags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Jump(u8),
    JumpIf(Condition, u8),
    JumpBa,
    Call(u8),
    CallZero(u8),
    Return,
    ReturnSkip,
    ReturnData(u8),
    Push(StackValue),
    Pop(StackValue),
    LoadY(u8),
    LoadX(u8),
    AddImmediate(Operand, u8),
    AdcImmediate(Operand, u8),
    SbcImmediate(Operand, u8),
    AndImmediate(Operand, u8),
    OrImmediate(Operand, u8),
    XorImmediate(Operand, u8),
    FanImmediate(Operand, u8),
    CompareImmediate(Operand, u8),
    Alu(AluOperation, Operand, Operand),
    AdcIndex(IndexNibble, u8),
    CompareIndex(IndexNibble, u8),
    RotateLeft(Operand),
    RotateRight(Operand),
    IncrementMemory(u8),
    DecrementMemory(u8),
    IncrementStackPointer,
    DecrementStackPointer,
    SetFlags(u8),
    ResetFlags(u8),
    AdcPostX(Operand),
    AdcPostY(Operand),
    SbcPostX(Operand),
    SbcPostY(Operand),
    LoadImmediate(Operand, u8),
    SetPage(u8),
    StoreIncrementX(u8),
    StoreIncrementY(u8),
    Load(Operand, Operand),
    LoadBytePostX(u8),
    LoadRegister(RegisterNibble, Operand),
    LoadFromRegister(Operand, RegisterNibble),
    StoreDirect(Operand, u8),
    LoadDirect(Operand, u8),
    LoadPostX(Operand, Operand),
    LoadPostY(Operand, Operand),
    Halt,
    Nop5,
    Nop7,
}

impl Instruction {
    /// Only explicitly supported opcode ranges; no dummy instructions.
    pub fn decode(opcode: u16) -> Option<Self> {
        match opcode {
            0x100..=0x1FF => Some(Self::ReturnData(opcode as u8)),
            0x400..=0x4FF => Some(Self::Call(opcode as u8)),
            0x500..=0x5FF => Some(Self::CallZero(opcode as u8)),
            0xFDE => Some(Self::ReturnSkip),
            0xFDF => Some(Self::Return),
            0x200..=0x2FF => Some(Self::JumpIf(Condition::CarrySet, opcode as u8)),
            0x300..=0x3FF => Some(Self::JumpIf(Condition::CarryClear, opcode as u8)),
            0x600..=0x6FF => Some(Self::JumpIf(Condition::ZeroSet, opcode as u8)),
            0x700..=0x7FF => Some(Self::JumpIf(Condition::ZeroClear, opcode as u8)),
            0xFE8 => Some(Self::JumpBa),
            0x000..=0x0FF => Some(Self::Jump(opcode as u8)),
            0x900..=0x9FF => Some(Self::LoadBytePostX(opcode as u8)),
            0xE80..=0xE8B | 0xE90..=0xE9B => {
                let field = match opcode & 0xFFC {
                    0xE80 => RegisterNibble::Xp,
                    0xE84 => RegisterNibble::Xh,
                    0xE88 => RegisterNibble::Xl,
                    0xE90 => RegisterNibble::Yp,
                    0xE94 => RegisterNibble::Yh,
                    0xE98 => RegisterNibble::Yl,
                    _ => unreachable!("restricted register load range"),
                };
                Some(Self::LoadRegister(field, Operand::from_bits(opcode)))
            }
            0xEA0..=0xEAB | 0xEB0..=0xEBB => {
                let field = match opcode & 0xFFC {
                    0xEA0 => RegisterNibble::Xp,
                    0xEA4 => RegisterNibble::Xh,
                    0xEA8 => RegisterNibble::Xl,
                    0xEB0 => RegisterNibble::Yp,
                    0xEB4 => RegisterNibble::Yh,
                    0xEB8 => RegisterNibble::Yl,
                    _ => unreachable!("restricted register read range"),
                };
                Some(Self::LoadFromRegister(Operand::from_bits(opcode), field))
            }
            0xEE0..=0xEEF => Some(Self::LoadPostX(
                Operand::from_bits(opcode >> 2),
                Operand::from_bits(opcode),
            )),
            0xEF0..=0xEFF => Some(Self::LoadPostY(
                Operand::from_bits(opcode >> 2),
                Operand::from_bits(opcode),
            )),
            0xF80..=0xF9F => Some(Self::StoreDirect(
                if opcode < 0xF90 {
                    Operand::A
                } else {
                    Operand::B
                },
                (opcode & 15) as u8,
            )),
            0xFA0..=0xFBF => Some(Self::LoadDirect(
                if opcode < 0xFB0 {
                    Operand::A
                } else {
                    Operand::B
                },
                (opcode & 15) as u8,
            )),
            0xFE0..=0xFE3 => Some(Self::LoadRegister(
                RegisterNibble::Sph,
                Operand::from_bits(opcode),
            )),
            0xFE4..=0xFE7 => Some(Self::LoadFromRegister(
                Operand::from_bits(opcode),
                RegisterNibble::Sph,
            )),
            0xFF0..=0xFF3 => Some(Self::LoadRegister(
                RegisterNibble::Spl,
                Operand::from_bits(opcode),
            )),
            0xFF4..=0xFF7 => Some(Self::LoadFromRegister(
                Operand::from_bits(opcode),
                RegisterNibble::Spl,
            )),
            0x800..=0x8FF => Some(Self::LoadY(opcode as u8)),
            0xA00..=0xA3F => Some(Self::AdcIndex(
                IndexNibble::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xA40..=0xA7F => Some(Self::CompareIndex(
                IndexNibble::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xA80..=0xAEF | 0xF00..=0xF1F => {
                let operation = match opcode & 0xFF0 {
                    0xA80 => AluOperation::Add,
                    0xA90 => AluOperation::Adc,
                    0xAA0 => AluOperation::Sub,
                    0xAB0 => AluOperation::Sbc,
                    0xAC0 => AluOperation::And,
                    0xAD0 => AluOperation::Or,
                    0xAE0 => AluOperation::Xor,
                    0xF00 => AluOperation::Compare,
                    0xF10 => AluOperation::Fan,
                    _ => unreachable!("outer match restricts this to two-operand ALU codes"),
                };
                Some(Self::Alu(
                    operation,
                    Operand::from_bits(opcode >> 2),
                    Operand::from_bits(opcode),
                ))
            }
            0xAF0..=0xAFF => Some(Self::RotateLeft(Operand::from_bits(opcode))),
            0xE8C..=0xE8F => Some(Self::RotateRight(Operand::from_bits(opcode))),
            0xB00..=0xBFF => Some(Self::LoadX(opcode as u8)),
            0xC00..=0xC3F => Some(Self::AddImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xC40..=0xC7F => Some(Self::AdcImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xC80..=0xCBF => Some(Self::AndImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xCC0..=0xCFF => Some(Self::OrImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xD00..=0xD3F => Some(Self::XorImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xD40..=0xD7F => Some(Self::SbcImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xD80..=0xDBF => Some(Self::FanImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xDC0..=0xDFF => Some(Self::CompareImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xE00..=0xE3F => Some(Self::LoadImmediate(
                Operand::from_bits(opcode >> 4),
                (opcode & 15) as u8,
            )),
            0xE40..=0xE5F => Some(Self::SetPage((opcode & 31) as u8)),
            0xE60..=0xE6F => Some(Self::StoreIncrementX((opcode & 15) as u8)),
            0xE70..=0xE7F => Some(Self::StoreIncrementY((opcode & 15) as u8)),
            0xEC0..=0xECF => Some(Self::Load(
                Operand::from_bits(opcode >> 2),
                Operand::from_bits(opcode),
            )),
            0xF28..=0xF2B => Some(Self::AdcPostX(Operand::from_bits(opcode))),
            0xF2C..=0xF2F => Some(Self::AdcPostY(Operand::from_bits(opcode))),
            0xF38..=0xF3B => Some(Self::SbcPostX(Operand::from_bits(opcode))),
            0xF3C..=0xF3F => Some(Self::SbcPostY(Operand::from_bits(opcode))),
            0xF40..=0xF4F => Some(Self::SetFlags((opcode & 15) as u8)),
            0xF50..=0xF5F => Some(Self::ResetFlags((opcode & 15) as u8)),
            0xF60..=0xF6F => Some(Self::IncrementMemory((opcode & 15) as u8)),
            0xF70..=0xF7F => Some(Self::DecrementMemory((opcode & 15) as u8)),
            0xFC0..=0xFCA | 0xFD0..=0xFDA => {
                let value = match opcode & 15 {
                    0..=3 => StackValue::Operand(Operand::from_bits(opcode)),
                    4..=9 => StackValue::Register(
                        [
                            RegisterNibble::Xp,
                            RegisterNibble::Xh,
                            RegisterNibble::Xl,
                            RegisterNibble::Yp,
                            RegisterNibble::Yh,
                            RegisterNibble::Yl,
                        ][((opcode & 15) - 4) as usize],
                    ),
                    10 => StackValue::Flags,
                    _ => unreachable!("restricted stack encoding range"),
                };
                Some(if opcode < 0xFD0 {
                    Self::Push(value)
                } else {
                    Self::Pop(value)
                })
            }
            0xFCB => Some(Self::DecrementStackPointer),
            0xFDB => Some(Self::IncrementStackPointer),
            0xFF8 => Some(Self::Halt),
            0xFFB => Some(Self::Nop5),
            0xFFF => Some(Self::Nop7),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpuError {
    ProgramTooLarge(usize),
    InvalidProgramWord { address: usize, value: u16 },
    MissingInstruction { pc: u16 },
    Halted { pc: u16 },
    UnsupportedOpcode { pc: u16, opcode: u16 },
    UnsupportedAddress(u16),
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramTooLarge(len) => write!(f, "program has {len} words; maximum is 8192"),
            Self::InvalidProgramWord { address, value } => {
                write!(
                    f,
                    "invalid 12-bit opcode {value:#06x} at word {address:#06x}"
                )
            }
            Self::Halted { pc } => {
                write!(
                    f,
                    "CPU halted at PC {pc:#06x}; interrupt wake-up is not implemented"
                )
            }
            Self::MissingInstruction { pc } => write!(f, "no instruction at PC {pc:#06x}"),
            Self::UnsupportedOpcode { pc, opcode } => {
                write!(f, "unsupported opcode {opcode:#05x} at PC {pc:#06x}")
            }
            Self::UnsupportedAddress(address) => {
                write!(f, "unsupported memory/peripheral address {address:#05x}")
            }
        }
    }
}

impl std::error::Error for CpuError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub pc: u16,
    pub opcode: u16,
    /// Instruction cycles returned by the Python handler, before oscillator conversion.
    pub cycles: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cpu {
    state: CpuState,
    program: Box<[u16]>,
    ram: [u8; RAM_NIBBLES],
    lcd: [u8; LCD_NIBBLES],
    io: Option<RefCell<Io>>,
    lcd_gap_bus: bool,
}

impl Cpu {
    /// Program words are already decoded 12-bit values. This is not a ROM file loader.
    /// Word 0 is at address 0; the reference initial PC is 0x100.
    pub fn new(program: &[u16]) -> Result<Self, CpuError> {
        if program.len() > PROGRAM_WORDS {
            return Err(CpuError::ProgramTooLarge(program.len()));
        }
        for (address, &value) in program.iter().enumerate() {
            if value > 0xFFF {
                return Err(CpuError::InvalidProgramWord { address, value });
            }
        }
        Ok(Self {
            state: CpuState::default(),
            program: program.into(),
            ram: [0; RAM_NIBBLES],
            lcd: [0; LCD_NIBBLES],
            io: None,
            lcd_gap_bus: false,
        })
    }

    pub(crate) fn reset_mcu(&mut self) {
        self.state = CpuState::default();
        self.ram.fill(0);
        self.lcd.fill(0);
        self.attach_io();
    }

    /// Genjintch/Angel MCU profiles use the reference policy for disconnected LCD
    /// addresses and unregistered F7C. BrickEmuPy reads these as zero
    /// and ignores writes. Keep the established standalone/P1 strict policy
    /// and reject every other gap.
    pub(crate) fn enable_lcd_gap_bus(&mut self) {
        self.lcd_gap_bus = true;
    }

    pub(crate) fn attach_io(&mut self) {
        self.io = Some(RefCell::new(Io::default()));
    }

    pub(crate) fn io(&self) -> Ref<'_, Io> {
        self.io.as_ref().expect("MCU mode requires I/O").borrow()
    }

    pub(crate) fn with_io<T>(&self, action: impl FnOnce(&mut Io) -> T) -> T {
        action(
            &mut self
                .io
                .as_ref()
                .expect("MCU mode requires I/O")
                .borrow_mut(),
        )
    }

    pub(crate) fn enter_interrupt(&mut self, vector: u8) {
        let pc = self.state.pc;
        let sp = self.state.sp;
        self.ram[sp.wrapping_sub(1) as usize] = ((pc >> 8) & 15) as u8;
        self.ram[sp.wrapping_sub(2) as usize] = ((pc >> 4) & 15) as u8;
        self.state.sp = sp.wrapping_sub(3);
        self.ram[self.state.sp as usize] = (pc & 15) as u8;
        self.state.interrupt_enable = false;
        self.state.halted = false;
        self.state.pc = (self.state.npc & 0x1000) | 0x100 | u16::from(vector);
        self.state.npc = self.state.pc;
    }

    pub fn state(&self) -> &CpuState {
        &self.state
    }

    pub fn ram(&self) -> &[u8; RAM_NIBBLES] {
        &self.ram
    }

    pub fn lcd_memory(&self) -> &[u8; LCD_NIBBLES] {
        &self.lcd
    }

    pub fn read_memory(&self, address: u16) -> Result<u8, CpuError> {
        match address {
            0x000..=0x2FF => Ok(self.ram[address as usize]),
            0xE00..=0xE4F => Ok(self.lcd[(address - 0xE00) as usize]),
            0xE80..=0xECF => Ok(self.lcd[(address - 0xE80 + 0x50) as usize]),
            0xE50..=0xE7F | 0xED0..=0xEFF | 0xF7C if self.lcd_gap_bus => Ok(0),
            0xF00..=0xF7E if self.io.is_some() && Io::supports(address) => {
                Ok(self.with_io(|io| io.read(address)))
            }
            _ => Err(CpuError::UnsupportedAddress(address)),
        }
    }

    pub fn write_memory(&mut self, address: u16, value: u8) -> Result<(), CpuError> {
        let value = value & 15;
        match address {
            0x000..=0x2FF => self.ram[address as usize] = value,
            0xE00..=0xE4F => self.lcd[(address - 0xE00) as usize] = value,
            0xE80..=0xECF => self.lcd[(address - 0xE80 + 0x50) as usize] = value,
            0xE50..=0xE7F | 0xED0..=0xEFF | 0xF7C if self.lcd_gap_bus => {}
            0xF00..=0xF7E if self.io.is_some() && Io::supports(address) => {
                self.with_io(|io| io.write(address, value))
            }
            _ => return Err(CpuError::UnsupportedAddress(address)),
        }
        Ok(())
    }

    fn read_operand(&self, operand: Operand) -> Result<u8, CpuError> {
        match operand {
            Operand::A => Ok(self.state.a),
            Operand::B => Ok(self.state.b),
            Operand::Mx => self.read_memory(self.state.ix),
            Operand::My => self.read_memory(self.state.iy),
        }
    }

    fn write_operand(&mut self, operand: Operand, value: u8) -> Result<(), CpuError> {
        match operand {
            Operand::A => self.state.a = value & 15,
            Operand::B => self.state.b = value & 15,
            Operand::Mx => self.write_memory(self.state.ix, value)?,
            Operand::My => self.write_memory(self.state.iy, value)?,
        }
        Ok(())
    }

    fn read_register_nibble(&self, field: RegisterNibble) -> u8 {
        let (value, shift) = match field {
            RegisterNibble::Xp => (self.state.ix, 8),
            RegisterNibble::Xh => (self.state.ix, 4),
            RegisterNibble::Xl => (self.state.ix, 0),
            RegisterNibble::Yp => (self.state.iy, 8),
            RegisterNibble::Yh => (self.state.iy, 4),
            RegisterNibble::Yl => (self.state.iy, 0),
            RegisterNibble::Sph => (u16::from(self.state.sp), 4),
            RegisterNibble::Spl => (u16::from(self.state.sp), 0),
        };
        ((value >> shift) & 15) as u8
    }

    fn write_register_nibble(&mut self, field: RegisterNibble, value: u8) {
        let (register, shift) = match field {
            RegisterNibble::Xp => (&mut self.state.ix, 8),
            RegisterNibble::Xh => (&mut self.state.ix, 4),
            RegisterNibble::Xl => (&mut self.state.ix, 0),
            RegisterNibble::Yp => (&mut self.state.iy, 8),
            RegisterNibble::Yh => (&mut self.state.iy, 4),
            RegisterNibble::Yl => (&mut self.state.iy, 0),
            RegisterNibble::Sph => {
                self.state.sp = (self.state.sp & 15) | ((value & 15) << 4);
                return;
            }
            RegisterNibble::Spl => {
                self.state.sp = (self.state.sp & 0xF0) | (value & 15);
                return;
            }
        };
        *register = (*register & !(15 << shift)) | (u16::from(value & 15) << shift);
    }

    /// Validate every destination before committing any of these ordinary memory writes.
    fn write_memory_batch(&mut self, writes: &[(u16, u8)]) -> Result<(), CpuError> {
        for &(address, _) in writes {
            self.validate_write_address(address)?;
        }
        for &(address, value) in writes {
            self.write_memory(address, value)?;
        }
        Ok(())
    }

    fn validate_write_address(&self, address: u16) -> Result<(), CpuError> {
        // Validation must not acknowledge a read-to-clear interrupt register.
        match address {
            0..=0x2FF | 0xE00..=0xE4F | 0xE80..=0xECF => Ok(()),
            0xE50..=0xE7F | 0xED0..=0xEFF | 0xF7C if self.lcd_gap_bus => Ok(()),
            0xF00..=0xF7E if self.io.is_some() && Io::supports(address) => Ok(()),
            _ => Err(CpuError::UnsupportedAddress(address)),
        }
    }

    fn write_byte_post_x(&mut self, value: u8) -> Result<(), CpuError> {
        let ix = self.state.ix;
        self.write_memory_batch(&[
            (ix, value & 15),
            ((ix & 0xF00) | ((ix + 1) & 0xFF), value >> 4),
        ])?;
        self.state.ix = (ix & 0xF00) | ((ix + 2) & 0xFF);
        Ok(())
    }

    fn stacked_return_address(&self) -> Result<u16, CpuError> {
        let sp = u16::from(self.state.sp);
        // The original reads SP+1 and SP+2 without eight-bit address wrapping.
        Ok((self.state.pc & 0x1000)
            | u16::from(self.read_memory(sp)?)
            | (u16::from(self.read_memory(sp + 1)?) << 4)
            | (u16::from(self.read_memory(sp + 2)?) << 8))
    }

    fn next_pc(&mut self) {
        self.state.pc = (self.state.pc & 0x1000) | ((self.state.pc + 1) & 0xFFF);
        self.state.npc = self.state.pc;
    }

    /// Executes one instruction; a halted CPU returns Halted without fetching or advancing.
    /// MCU idle cycles, interrupt wake-up, and timers are outside this instruction API.
    pub fn step(&mut self) -> Result<Step, CpuError> {
        // CPU writes already commit atomically. Also roll back any read-to-clear
        // I/O effects when a later operand or destination is invalid.
        let saved_io = self.io.as_ref().map(|io| io.borrow().clone());
        let result = self.step_instruction();
        if result.is_err()
            && let Some(saved) = saved_io
        {
            self.with_io(|io| *io = saved);
        }
        result
    }

    fn step_instruction(&mut self) -> Result<Step, CpuError> {
        if self.state.halted {
            return Err(CpuError::Halted { pc: self.state.pc });
        }
        let pc = self.state.pc;
        let fetch = if self.io.is_some() && !self.program.is_empty() {
            usize::from(pc) % self.program.len()
        } else {
            usize::from(pc)
        };
        let opcode = *self
            .program
            .get(fetch)
            .ok_or(CpuError::MissingInstruction { pc })?;
        let instruction =
            Instruction::decode(opcode).ok_or(CpuError::UnsupportedOpcode { pc, opcode })?;
        // Keep delay changes local until the instruction has completed successfully.
        let mut next_interrupt_delay = matches!(instruction, Instruction::SetPage(_));
        let cycles = match instruction {
            Instruction::JumpIf(condition, target) => {
                let taken = match condition {
                    Condition::CarrySet => self.state.carry,
                    Condition::CarryClear => !self.state.carry,
                    Condition::ZeroSet => self.state.zero,
                    Condition::ZeroClear => !self.state.zero,
                };
                if taken {
                    self.state.pc = (self.state.npc & 0x1F00) | u16::from(target);
                } else {
                    self.next_pc();
                }
                5
            }
            Instruction::JumpBa => {
                self.state.pc = (self.state.npc & 0x1F00)
                    | (u16::from(self.state.b) << 4)
                    | u16::from(self.state.a);
                5
            }
            Instruction::Jump(target) => {
                self.state.pc = (self.state.npc & 0x1F00) | u16::from(target);
                // _jp_s explicitly leaves _NPC unchanged.
                5
            }
            Instruction::SetPage(page) => {
                self.state.npc = u16::from(page) << 8;
                self.state.pc = (pc & 0x1000) | ((pc + 1) & 0xFFF);
                5
            }
            Instruction::Call(target) | Instruction::CallZero(target) => {
                let sp = self.state.sp;
                let return_pc = pc + 1;
                self.write_memory_batch(&[
                    (u16::from(sp.wrapping_sub(1)), ((return_pc >> 8) & 15) as u8),
                    (u16::from(sp.wrapping_sub(2)), ((return_pc >> 4) & 15) as u8),
                    (u16::from(sp.wrapping_sub(3)), (return_pc & 15) as u8),
                ])?;
                self.state.sp = sp.wrapping_sub(3);
                if matches!(instruction, Instruction::CallZero(_)) {
                    self.state.pc = (self.state.npc & 0x1000) | u16::from(target);
                    self.state.npc = self.state.pc;
                } else {
                    self.state.pc = (self.state.npc & 0x1F00) | u16::from(target);
                }
                7
            }
            Instruction::Return | Instruction::ReturnSkip | Instruction::ReturnData(_) => {
                // Read the complete address before a RETD store can alias stack memory.
                let return_pc = self.stacked_return_address()?;
                if let Instruction::ReturnData(value) = instruction {
                    // Preflight both writes before committing PC, SP, or the first nibble.
                    self.write_byte_post_x(value)?;
                }
                self.state.sp = self.state.sp.wrapping_add(3);
                self.state.pc = return_pc;
                if matches!(instruction, Instruction::ReturnSkip) {
                    self.next_pc();
                } else {
                    self.state.npc = return_pc;
                }
                if matches!(instruction, Instruction::Return) {
                    7
                } else {
                    12
                }
            }
            Instruction::Push(source) => {
                let value = match source {
                    StackValue::Operand(operand) => self.read_operand(operand)?,
                    StackValue::Register(field) => self.read_register_nibble(field),
                    StackValue::Flags => {
                        u8::from(self.state.carry)
                            | (u8::from(self.state.zero) << 1)
                            | (u8::from(self.state.decimal) << 2)
                            | (u8::from(self.state.interrupt_enable) << 3)
                    }
                };
                let sp = self.state.sp.wrapping_sub(1);
                self.write_memory(u16::from(sp), value)?;
                self.state.sp = sp;
                self.next_pc();
                5
            }
            Instruction::Pop(target) => {
                let value = self.read_memory(u16::from(self.state.sp))?;
                match target {
                    StackValue::Operand(operand) => self.write_operand(operand, value)?,
                    StackValue::Register(field) => self.write_register_nibble(field, value),
                    StackValue::Flags => {
                        let enabled = value & 8 != 0;
                        next_interrupt_delay = enabled && !self.state.interrupt_enable;
                        self.state.carry = value & 1 != 0;
                        self.state.zero = value & 2 != 0;
                        self.state.decimal = value & 4 != 0;
                        self.state.interrupt_enable = enabled;
                    }
                }
                self.state.sp = self.state.sp.wrapping_add(1);
                self.next_pc();
                5
            }
            Instruction::LoadBytePostX(value) => {
                self.write_byte_post_x(value)?;
                self.next_pc();
                5
            }
            Instruction::LoadRegister(field, source) => {
                // Read through the old pointer before changing any part of that pointer.
                let value = self.read_operand(source)?;
                self.write_register_nibble(field, value);
                self.next_pc();
                5
            }
            Instruction::LoadFromRegister(target, field) => {
                let value = self.read_register_nibble(field);
                self.write_operand(target, value)?;
                self.next_pc();
                5
            }
            Instruction::StoreDirect(source, address) => {
                let value = self.read_operand(source)?;
                self.write_memory(u16::from(address), value)?;
                self.next_pc();
                5
            }
            Instruction::LoadDirect(target, address) => {
                let value = self.read_memory(u16::from(address))?;
                self.write_operand(target, value)?;
                self.next_pc();
                5
            }
            Instruction::LoadPostX(target, source) | Instruction::LoadPostY(target, source) => {
                let value = self.read_operand(source)?;
                self.write_operand(target, value)?;
                if matches!(instruction, Instruction::LoadPostX(_, _)) {
                    self.state.ix = (self.state.ix & 0xF00) | ((self.state.ix + 1) & 0xFF);
                } else {
                    self.state.iy = (self.state.iy & 0xF00) | ((self.state.iy + 1) & 0xFF);
                }
                self.next_pc();
                5
            }
            Instruction::LoadX(value) => {
                self.state.ix = (self.state.ix & 0xF00) | u16::from(value);
                self.next_pc();
                5
            }
            Instruction::LoadY(value) => {
                self.state.iy = (self.state.iy & 0xF00) | u16::from(value);
                self.next_pc();
                5
            }
            Instruction::LoadImmediate(target, value) => {
                self.write_operand(target, value)?;
                self.next_pc();
                5
            }
            Instruction::Load(target, source) => {
                let value = self.read_operand(source)?;
                self.write_operand(target, value)?;
                self.next_pc();
                5
            }
            Instruction::StoreIncrementX(value) => {
                self.write_memory(self.state.ix, value)?;
                self.state.ix = (self.state.ix & 0xF00) | ((self.state.ix + 1) & 0xFF);
                self.next_pc();
                5
            }
            Instruction::StoreIncrementY(value) => {
                self.write_memory(self.state.iy, value)?;
                self.state.iy = (self.state.iy & 0xF00) | ((self.state.iy + 1) & 0xFF);
                self.next_pc();
                5
            }
            Instruction::AddImmediate(target, value) => {
                let mut result = self.read_operand(target)? + value;
                let mut carry = result > 15;
                if self.state.decimal && result > 9 {
                    result += 6;
                    carry = true;
                }
                self.write_operand(target, result)?;
                self.state.carry = carry;
                self.state.zero = result & 15 == 0;
                self.next_pc();
                7
            }
            Instruction::AdcImmediate(target, value) => {
                let mut result = self.read_operand(target)? + value + u8::from(self.state.carry);
                let mut carry = result > 15;
                if self.state.decimal && result > 9 {
                    result += 6;
                    carry = true;
                }
                self.write_operand(target, result)?;
                self.state.carry = carry;
                self.state.zero = result & 15 == 0;
                self.next_pc();
                7
            }
            Instruction::SbcImmediate(target, value) => {
                // Python: C represents borrow; decimal correction adds 10 exactly once.
                // Invalid BCD nibbles can still leave a negative result.
                let mut result = i16::from(self.read_operand(target)?)
                    - i16::from(value)
                    - i16::from(self.state.carry);
                let carry = result < 0;
                if self.state.decimal && carry {
                    result += 10;
                }
                let nibble = (result & 15) as u8;
                self.write_operand(target, nibble)?;
                self.state.carry = carry;
                self.state.zero = nibble == 0;
                self.next_pc();
                7
            }
            Instruction::AndImmediate(target, value)
            | Instruction::OrImmediate(target, value)
            | Instruction::XorImmediate(target, value) => {
                let operand = self.read_operand(target)?;
                let result = match instruction {
                    Instruction::AndImmediate(_, _) => operand & value,
                    Instruction::OrImmediate(_, _) => operand | value,
                    Instruction::XorImmediate(_, _) => operand ^ value,
                    _ => unreachable!("outer match restricts this to AND/OR/XOR"),
                };
                self.write_operand(target, result)?;
                self.state.zero = result == 0;
                self.next_pc();
                7
            }
            Instruction::FanImmediate(target, value) => {
                self.state.zero = self.read_operand(target)? & value == 0;
                self.next_pc();
                7
            }
            Instruction::CompareImmediate(target, value) => {
                let operand = self.read_operand(target)?;
                self.state.zero = operand == value;
                self.state.carry = operand < value;
                self.next_pc();
                7
            }
            Instruction::Alu(operation, target, source) => {
                // Read both values before any change, including r=q / M(X)=M(Y).
                let left = self.read_operand(target)?;
                let right = self.read_operand(source)?;
                let (result, carry, zero) = match operation {
                    AluOperation::Add | AluOperation::Adc => {
                        let carry_in = operation == AluOperation::Adc && self.state.carry;
                        let mut sum = left + right + u8::from(carry_in);
                        let mut carry = sum > 15;
                        if self.state.decimal && sum > 9 {
                            sum += 6;
                            carry = true;
                        }
                        let nibble = sum & 15;
                        (Some(nibble), Some(carry), nibble == 0)
                    }
                    AluOperation::Sub | AluOperation::Sbc => {
                        let borrow_in = operation == AluOperation::Sbc && self.state.carry;
                        let mut difference =
                            i16::from(left) - i16::from(right) - i16::from(borrow_in);
                        let borrow = difference < 0;
                        if self.state.decimal && borrow {
                            difference += 10;
                        }
                        let nibble = (difference & 15) as u8;
                        (Some(nibble), Some(borrow), nibble == 0)
                    }
                    AluOperation::And | AluOperation::Or | AluOperation::Xor => {
                        let result = match operation {
                            AluOperation::And => left & right,
                            AluOperation::Or => left | right,
                            AluOperation::Xor => left ^ right,
                            _ => unreachable!("outer match restricts this to logic operations"),
                        };
                        (Some(result), None, result == 0)
                    }
                    AluOperation::Compare => (None, Some(left < right), left == right),
                    AluOperation::Fan => (None, None, left & right == 0),
                };
                // Complete the write before committing flags and PC.
                if let Some(value) = result {
                    self.write_operand(target, value)?;
                }
                if let Some(value) = carry {
                    self.state.carry = value;
                }
                self.state.zero = zero;
                self.next_pc();
                7
            }
            Instruction::AdcIndex(target, immediate)
            | Instruction::CompareIndex(target, immediate) => {
                let address = match target {
                    IndexNibble::Xh | IndexNibble::Xl => self.state.ix,
                    IndexNibble::Yh | IndexNibble::Yl => self.state.iy,
                };
                let shift = match target {
                    IndexNibble::Xh | IndexNibble::Yh => 4,
                    IndexNibble::Xl | IndexNibble::Yl => 0,
                };
                let nibble = (address >> shift) & 15;
                if matches!(instruction, Instruction::AdcIndex(_, _)) {
                    // Index ADC is binary even with D=1. No carry into the adjacent nibble.
                    let sum = nibble + u16::from(immediate) + u16::from(self.state.carry);
                    let updated = (address & !(15 << shift)) | ((sum & 15) << shift);
                    match target {
                        IndexNibble::Xh | IndexNibble::Xl => self.state.ix = updated,
                        IndexNibble::Yh | IndexNibble::Yl => self.state.iy = updated,
                    }
                    self.state.zero = sum & 15 == 0;
                    self.state.carry = sum > 15;
                } else {
                    self.state.zero = nibble == u16::from(immediate);
                    self.state.carry = nibble < u16::from(immediate);
                }
                self.next_pc();
                7
            }
            Instruction::AdcPostX(source)
            | Instruction::AdcPostY(source)
            | Instruction::SbcPostX(source)
            | Instruction::SbcPostY(source) => {
                let is_x = matches!(
                    instruction,
                    Instruction::AdcPostX(_) | Instruction::SbcPostX(_)
                );
                let subtract = matches!(
                    instruction,
                    Instruction::SbcPostX(_) | Instruction::SbcPostY(_)
                );
                let address = if is_x { self.state.ix } else { self.state.iy };
                // Read both operands through the old pointers, including aliases.
                let left = self.read_memory(address)?;
                let right = self.read_operand(source)?;
                let (result, carry) = if subtract {
                    let mut difference =
                        i16::from(left) - i16::from(right) - i16::from(self.state.carry);
                    let borrow = difference < 0;
                    if self.state.decimal && borrow {
                        difference += 10;
                    }
                    ((difference & 15) as u8, borrow)
                } else {
                    let mut sum = left + right + u8::from(self.state.carry);
                    let mut carry = sum > 15;
                    if self.state.decimal && sum > 9 {
                        sum += 6;
                        carry = true;
                    }
                    (sum & 15, carry)
                };
                // Commit only after the old destination was successfully written.
                self.write_memory(address, result)?;
                let next_address = (address & 0xF00) | ((address + 1) & 0xFF);
                if is_x {
                    self.state.ix = next_address;
                } else {
                    self.state.iy = next_address;
                }
                self.state.carry = carry;
                self.state.zero = result == 0;
                self.next_pc();
                7
            }
            Instruction::RotateLeft(target) | Instruction::RotateRight(target) => {
                let value = self.read_operand(target)?;
                let (result, carry, cycles) = if matches!(instruction, Instruction::RotateLeft(_)) {
                    let extended = (value << 1) | u8::from(self.state.carry);
                    (extended & 15, extended > 15, 7)
                } else {
                    (
                        (value >> 1) | (u8::from(self.state.carry) << 3),
                        value & 1 != 0,
                        5,
                    )
                };
                // A failed access must not change carry, PC, or any preserved flags.
                self.write_operand(target, result)?;
                self.state.carry = carry;
                // Unlike ALU instructions, rotations preserve Z as well as D/I.
                self.next_pc();
                cycles
            }
            Instruction::IncrementMemory(address) | Instruction::DecrementMemory(address) => {
                // The immediate nibble is a direct RAM address, independent of IX/IY.
                let old = i16::from(self.read_memory(u16::from(address))?);
                let increment = matches!(instruction, Instruction::IncrementMemory(_));
                let result = if increment { old + 1 } else { old - 1 };
                self.write_memory(u16::from(address), (result & 15) as u8)?;
                self.state.zero = if increment { result == 16 } else { result == 0 };
                self.state.carry = if increment { result > 15 } else { result < 0 };
                self.next_pc();
                7
            }
            Instruction::IncrementStackPointer | Instruction::DecrementStackPointer => {
                // SP arithmetic wraps at eight bits and preserves every flag.
                self.state.sp = if matches!(instruction, Instruction::IncrementStackPointer) {
                    self.state.sp.wrapping_add(1)
                } else {
                    self.state.sp.wrapping_sub(1)
                };
                self.next_pc();
                5
            }
            Instruction::SetFlags(mask) => {
                // Delay applies only to a new I=0 -> I=1 transition.
                next_interrupt_delay = mask & 8 != 0 && !self.state.interrupt_enable;
                self.state.carry |= mask & 1 != 0;
                self.state.zero |= mask & 2 != 0;
                self.state.decimal |= mask & 4 != 0;
                self.state.interrupt_enable |= mask & 8 != 0;
                self.next_pc();
                7
            }
            Instruction::ResetFlags(mask) => {
                // RST uses a keep mask (F &= i), not an inverted clear mask.
                self.state.carry &= mask & 1 != 0;
                self.state.zero &= mask & 2 != 0;
                self.state.decimal &= mask & 4 != 0;
                self.state.interrupt_enable &= mask & 8 != 0;
                self.next_pc();
                7
            }
            Instruction::Halt => {
                self.state.halted = true;
                self.next_pc();
                5
            }
            Instruction::Nop5 => {
                self.next_pc();
                5
            }
            Instruction::Nop7 => {
                self.next_pc();
                7
            }
        };
        self.state.interrupt_delay = next_interrupt_delay;
        Ok(Step { pc, opcode, cycles })
    }
}

#[cfg(test)]
#[path = "e0c6200_tests.rs"]
mod tests;

impl CpuState {
    pub(crate) fn encode_state(&self, e: &mut crate::core::state::Encoder) {
        e.u8(self.a);
        e.u8(self.b);
        e.u16(self.ix);
        e.u16(self.iy);
        e.u8(self.sp);
        e.u16(self.pc);
        e.u16(self.npc);
        e.bool(self.carry);
        e.bool(self.zero);
        e.bool(self.decimal);
        e.bool(self.interrupt_enable);
        e.bool(self.interrupt_delay);
        e.bool(self.halted);
    }
    pub(crate) fn decode_state(
        &mut self,
        d: &mut crate::core::state::Decoder<'_>,
    ) -> crate::core::state::Result<()> {
        self.a = d.u8()?;
        self.b = d.u8()?;
        self.ix = d.u16()?;
        self.iy = d.u16()?;
        self.sp = d.u8()?;
        self.pc = d.u16()?;
        self.npc = d.u16()?;
        self.carry = d.bool()?;
        self.zero = d.bool()?;
        self.decimal = d.bool()?;
        self.interrupt_enable = d.bool()?;
        self.interrupt_delay = d.bool()?;
        self.halted = d.bool()?;
        crate::core::state::require(
            self.a <= 15
                && self.b <= 15
                && self.ix <= 0xFFF
                && self.iy <= 0xFFF
                && self.pc <= 0x1FFF
                && self.npc <= 0x1FFF,
            "Invalid CPU registers",
        )?;
        Ok(())
    }
}

impl Cpu {
    pub(crate) fn program_words(&self) -> &[u16] {
        &self.program
    }
    pub(crate) fn encode_state(&self, e: &mut crate::core::state::Encoder) {
        self.state.encode_state(e);
        e.bytes(&self.ram);
        e.bytes(&self.lcd);
        self.io().encode_state(e);
    }
    pub(crate) fn decode_state(
        &mut self,
        d: &mut crate::core::state::Decoder<'_>,
    ) -> crate::core::state::Result<()> {
        self.state.decode_state(d)?;
        self.ram = d.array()?;
        self.lcd = d.array()?;
        crate::core::state::require(
            self.ram.iter().chain(&self.lcd).all(|&v| v <= 15),
            "Invalid RAM/LCD nibble",
        )?;
        let mut io = self.io().clone();
        io.decode_state(d)?;
        self.with_io(|v| *v = io);
        Ok(())
    }
}
