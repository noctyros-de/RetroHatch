// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros (Rust translation and integration)
// Derived from maragotchi/tamaemu; retain upstream GPL notices and provenance.
//! Little-endian memory and AMD-style NOR bus translated from tamaemu/src/mem.c.
use super::{lcd::Lcd, peripherals::Peripherals, profile::HardwareProfile};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusError {
    PeripheralNotImplemented(u32),
    InvalidRomSize { expected: usize, actual: usize },
}
impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PeripheralNotImplemented(a) => {
                write!(f, "S1C33 peripheral at {a:#010x} is not implemented")
            }
            Self::InvalidRomSize { expected, actual } => write!(
                f,
                "S1C33 image has {actual} bytes; hardware profile requires {expected}"
            ),
        }
    }
}
impl std::error::Error for BusError {}
pub trait Bus {
    fn read8(&mut self, address: u32) -> Result<u8, BusError>;
    fn read16(&mut self, address: u32) -> Result<u16, BusError>;
    fn read32(&mut self, address: u32) -> Result<u32, BusError>;
    fn write8(&mut self, address: u32, value: u8) -> Result<(), BusError>;
    fn write16(&mut self, address: u32, value: u16) -> Result<(), BusError>;
    fn write32(&mut self, address: u32, value: u32) -> Result<(), BusError>;
    fn executable(&self, address: u32) -> bool;
    fn cpu_cycle(&mut self, _cycles: u64) {}
    fn begin(&mut self);
    fn commit(&mut self);
    fn rollback(&mut self);
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlashState {
    pub command: u8,
    pub autoselect: bool,
    pub busy_reads: u8,
    pub operation_data: u16,
    pub toggle: bool,
    pub dirty: bool,
    pub programs: u64,
    pub erases: u64,
}
#[derive(Clone)]
struct Undo {
    region: u8,
    offset: usize,
    data: Vec<u8>,
}
#[derive(Clone)]
struct Transaction {
    flash: FlashState,
    last_io: u32,
    undo: Vec<Undo>,
    lcd: Option<Lcd>,
    peripherals: Option<Peripherals>,
}
#[derive(Clone)]
pub struct Memory {
    pub profile: HardwareProfile,
    flash: Vec<u8>,
    pub a0ram: Vec<u8>,
    pub ivram: Vec<u8>,
    pub dstram: Vec<u8>,
    pub lcd: Lcd,
    pub peripherals: Option<Peripherals>,
    pub flash_state: FlashState,
    pub last_io_read: u32,
    transaction: Option<Transaction>,
    cpu_cycle: u64,
}
impl Memory {
    pub fn new(profile: HardwareProfile, image: &[u8]) -> Result<Self, BusError> {
        if image.len() != profile.rom_size as usize {
            return Err(BusError::InvalidRomSize {
                expected: profile.rom_size as usize,
                actual: image.len(),
            });
        }
        Ok(Self {
            profile,
            flash: image.to_vec(),
            a0ram: vec![0; profile.a0ram_size as usize],
            ivram: vec![0; profile.ivram_size as usize],
            dstram: vec![0; profile.dstram_size as usize],
            lcd: Lcd::default(),
            peripherals: None,
            flash_state: FlashState::default(),
            last_io_read: 0,
            transaction: None,
            cpu_cycle: 0,
        })
    }
    pub fn flash(&self) -> &[u8] {
        &self.flash
    }
    fn offset(a: u32, base: u32, size: u32, width: u32) -> Option<usize> {
        let off = a.wrapping_sub(base);
        if width <= size && off <= size - width {
            Some(off as usize)
        } else {
            None
        }
    }
    fn region(&self, region: u8) -> &[u8] {
        match region {
            0 => &self.flash,
            1 => &self.a0ram,
            2 => &self.ivram,
            3 => &self.dstram,
            _ => unreachable!(),
        }
    }
    fn region_mut(&mut self, region: u8) -> &mut [u8] {
        match region {
            0 => &mut self.flash,
            1 => &mut self.a0ram,
            2 => &mut self.ivram,
            3 => &mut self.dstram,
            _ => unreachable!(),
        }
    }
    fn store(&mut self, region: u8, offset: usize, data: &[u8]) {
        if self.transaction.is_some() {
            let old = self.region(region)[offset..offset + data.len()].to_vec();
            self.transaction.as_mut().unwrap().undo.push(Undo {
                region,
                offset,
                data: old,
            });
        }
        self.region_mut(region)[offset..offset + data.len()].copy_from_slice(data);
    }
    fn ram_offset(&self, a: u32, width: u32) -> Option<(u8, usize)> {
        let p = self.profile;
        for (region, base, size) in [
            (1, p.a0ram_base, p.a0ram_size),
            (2, p.ivram_base, p.ivram_size),
            (3, p.dstram_base, p.dstram_size),
        ] {
            if let Some(offset) = Self::offset(a, base, size, width) {
                return Some((region, offset));
            }
        }
        None
    }
    fn status(&mut self) -> u16 {
        let f = &mut self.flash_state;
        f.toggle = !f.toggle;
        let result = if f.operation_data & 0x80 == 0 {
            0x80
        } else {
            0
        } | if f.toggle { 0x40 } else { 0 };
        f.busy_reads -= 1;
        result
    }
    fn flash_byte(&mut self, off: usize) -> u8 {
        if self.flash_state.busy_reads != 0 {
            return self.status() as u8;
        }
        if self.flash_state.autoselect {
            return if (off >> 1) & 255 == 0 {
                if off & 1 == 0 { 1 } else { 0 }
            } else if off & 1 == 0 {
                0xf6
            } else {
                0x22
            };
        }
        self.flash[off]
    }
    fn flash_write(&mut self, off: usize, value: u16, width: usize) {
        let data = value as u8;
        if self.flash_state.command == 3 {
            let mut final_value = 0;
            for i in 0..width {
                let at = (off + i) & (self.flash.len() - 1);
                let byte = self.flash[at] & (value >> (8 * i)) as u8;
                self.store(0, at, &[byte]);
                final_value |= (byte as u16) << (8 * i);
            }
            let f = &mut self.flash_state;
            f.programs = f.programs.wrapping_add(1);
            f.command = 0;
            f.dirty = true;
            f.busy_reads = 3;
            f.operation_data = if width == 2 {
                final_value
            } else {
                0xff00 | final_value
            };
            f.toggle = false;
            return;
        }
        if data == 0xf0 {
            self.flash_state.command = 0;
            self.flash_state.autoselect = false;
            return;
        }
        self.flash_state.command = match self.flash_state.command {
            0 => {
                if data == 0xaa {
                    1
                } else {
                    0
                }
            }
            1 => {
                if data == 0x55 {
                    2
                } else {
                    0
                }
            }
            2 => match data {
                0xa0 => 3,
                0x80 => 4,
                0x90 => {
                    self.flash_state.autoselect = true;
                    0
                }
                _ => 0,
            },
            4 => {
                if data == 0xaa {
                    5
                } else {
                    0
                }
            }
            5 => {
                if data == 0x55 {
                    6
                } else {
                    0
                }
            }
            6 => {
                if data == 0x30 {
                    let small = if self.profile.flash_top_boot {
                        off >= self.flash.len() - 0x10000
                    } else {
                        off < 0x10000
                    };
                    let size = if small { 0x2000 } else { 0x10000 };
                    let base = off & !(size - 1);
                    self.store(0, base, &vec![255; size]);
                    let f = &mut self.flash_state;
                    f.erases = f.erases.wrapping_add(1);
                    f.dirty = true;
                    f.busy_reads = 3;
                    f.operation_data = 0xffff;
                    f.toggle = false;
                } else if data == 0x10 {
                    self.store(0, 0, &vec![255; self.flash.len()]);
                    self.flash_state.dirty = true;
                }
                0
            }
            _ => 0,
        };
    }
    fn save_peripherals(&mut self) {
        if self
            .transaction
            .as_ref()
            .is_some_and(|t| t.peripherals.is_none())
        {
            self.transaction.as_mut().unwrap().peripherals = self.peripherals.clone();
        }
    }
    fn save_lcd(&mut self) {
        if self.transaction.as_ref().is_some_and(|t| t.lcd.is_none()) {
            self.transaction.as_mut().unwrap().lcd = Some(self.lcd.clone());
        }
    }
}
impl Bus for Memory {
    fn cpu_cycle(&mut self, cycles: u64) {
        self.cpu_cycle = cycles;
    }
    fn read8(&mut self, a: u32) -> Result<u8, BusError> {
        let p = self.profile;
        if let Some(off) = Self::offset(a, p.rom_base, p.rom_size, 1) {
            return Ok(self.flash_byte(off));
        }
        if let Some((region, off)) = self.ram_offset(a, 1) {
            return Ok(self.region(region)[off]);
        }
        if Self::offset(a, p.io_base, p.io_end - p.io_base, 1).is_some() {
            self.save_peripherals();
            if let Some(periph) = &mut self.peripherals {
                self.last_io_read = a;
                return Ok(periph.read8(a, p));
            }
            return Err(BusError::PeripheralNotImplemented(a));
        }
        self.last_io_read = a;
        // The reference explicitly returns zero for LCD status and unmapped data reads.
        Ok(0)
    }
    #[inline]
    fn read16(&mut self, a: u32) -> Result<u16, BusError> {
        let p = self.profile;
        if let Some(off) = Self::offset(a, p.rom_base, p.rom_size, 2) {
            if self.flash_state.busy_reads > 0 {
                return Ok(self.status());
            }
            if !self.flash_state.autoselect {
                return Ok(u16::from_le_bytes(
                    self.flash[off..off + 2].try_into().unwrap(),
                ));
            }
        }
        let low = self.read8(a)?;
        let high = self.read8(a.wrapping_add(1))?;
        Ok(u16::from_le_bytes([low, high]))
    }
    fn read32(&mut self, a: u32) -> Result<u32, BusError> {
        let p = self.profile;
        if Self::offset(a, p.io_base, p.io_end - p.io_base, 4).is_some()
            && self.peripherals.is_some()
        {
            self.save_peripherals();
            self.last_io_read = a;
            return Ok(self.peripherals.as_mut().unwrap().read32(a, p));
        }
        if let Some(off) = Self::offset(a, p.rom_base, p.rom_size, 4) {
            if self.flash_state.busy_reads == 0 && !self.flash_state.autoselect {
                return Ok(u32::from_le_bytes(
                    self.flash[off..off + 4].try_into().unwrap(),
                ));
            }
        }
        if let Some((region, off)) = self.ram_offset(a, 4) {
            return Ok(u32::from_le_bytes(
                self.region(region)[off..off + 4].try_into().unwrap(),
            ));
        }
        // Match the MSVC C reference's operand evaluation order. The C expression
        // does not sequence the halfword reads; this matters for NOR status polls.
        let high = self.read16(a.wrapping_add(2))? as u32;
        let low = self.read16(a)? as u32;
        Ok(low | (high << 16))
    }
    fn write8(&mut self, a: u32, value: u8) -> Result<(), BusError> {
        let p = self.profile;
        if let Some((region, off)) = self.ram_offset(a, 1) {
            self.store(region, off, &[value]);
            return Ok(());
        }
        if a == p.lcd_cmd_addr {
            self.save_lcd();
            self.lcd.write_command(value);
            return Ok(());
        }
        if a == p.lcd_data_addr {
            self.save_lcd();
            self.lcd.write_data(value);
            return Ok(());
        }
        if Self::offset(a, p.io_base, p.io_end - p.io_base, 1).is_some() {
            self.save_peripherals();
            if let Some(periph) = &mut self.peripherals {
                periph.audio_until(self.cpu_cycle);
                periph.write8(a, value, p);
                return Ok(());
            }
            return Err(BusError::PeripheralNotImplemented(a));
        }
        if let Some(off) = Self::offset(a, p.rom_base, p.rom_size, 1) {
            self.flash_write(off, value as u16, 1);
        }
        // Unmapped writes are ignored in the reference memory decoder.
        Ok(())
    }
    fn write16(&mut self, a: u32, value: u16) -> Result<(), BusError> {
        let p = self.profile;
        if let Some(off) = Self::offset(a, p.rom_base, p.rom_size, 2) {
            self.flash_write(off, value, 2);
            return Ok(());
        }
        self.write8(a, value as u8)?;
        self.write8(a.wrapping_add(1), (value >> 8) as u8)
    }
    fn write32(&mut self, a: u32, value: u32) -> Result<(), BusError> {
        let p = self.profile;
        if Self::offset(a, p.io_base, p.io_end - p.io_base, 4).is_some()
            && self.peripherals.is_some()
        {
            self.save_peripherals();
            let periph = self.peripherals.as_mut().unwrap();
            periph.audio_until(self.cpu_cycle);
            periph.write32(a, value, p);
            return Ok(());
        }
        self.write16(a, value as u16)?;
        self.write16(a.wrapping_add(2), (value >> 16) as u16)
    }
    #[inline]
    fn executable(&self, a: u32) -> bool {
        let p = self.profile;
        Self::offset(a, p.rom_base, p.rom_size, 1).is_some() || self.ram_offset(a, 1).is_some()
    }
    #[inline]
    fn begin(&mut self) {
        assert!(self.transaction.is_none(), "Nested CPU bus transaction");
        self.transaction = Some(Transaction {
            flash: self.flash_state,
            last_io: self.last_io_read,
            undo: vec![],
            lcd: None,
            peripherals: None,
        });
    }
    #[inline]
    fn commit(&mut self) {
        self.transaction = None;
    }
    fn rollback(&mut self) {
        if let Some(t) = self.transaction.take() {
            for undo in t.undo.into_iter().rev() {
                self.region_mut(undo.region)[undo.offset..undo.offset + undo.data.len()]
                    .copy_from_slice(&undo.data);
            }
            self.flash_state = t.flash;
            self.last_io_read = t.last_io;
            if let Some(peripherals) = t.peripherals {
                self.peripherals = Some(peripherals);
            }
            if let Some(lcd) = t.lcd {
                self.lcd = lcd;
            }
        }
    }
}
