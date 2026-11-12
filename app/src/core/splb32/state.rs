// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Versioned SPLB32 schema 6, profile 26. No raw struct layout or firmware bytes.
use super::*;
use crate::core::state::{Decoder, Encoder, MAGIC, sha256};
impl Machine {
    pub fn save_state(&self) -> Result<Vec<u8>> {
        require(
            self.pcm.len() <= 88200,
            "Drain the PCM queue before capturing oversized state",
        )?;
        let mut w = Encoder(Vec::new());
        w.bytes(MAGIC);
        w.u16(1);
        w.u16(6);
        w.u8(26);
        w.bytes(&sha256(&self.rom));
        w.u64(self.pc as u64);
        w.u64(self.a as u64);
        w.u64(self.x as u64);
        w.u64(self.y as u64);
        w.u64(self.sp as u64);
        w.u64(self.nf as u64);
        w.u64(self.vf as u64);
        w.u64(self.bf as u64);
        w.u64(self.df as u64);
        w.u64(self.iflag as u64);
        w.u64(self.zf as u64);
        w.u64(self.cf as u64);
        w.u64(self.rom_bank as u64);
        w.u64(self.cpu_enbl as u64);
        w.u64(self.cpu_div as u64);
        w.u64(self.clk32768_enable as u64);
        w.u64(self.lcd_ctrl1 as u64);
        w.u64(self.int_ctrl as u64);
        w.u64(self.ireq as u64);
        w.u64(self.nmi_ctrl as u64);
        w.u64(self.nmireq as u64);
        w.u64(self.wakeup_ctrl as u64);
        w.u64(self.wakeupreq as u64);
        w.u64(self.time_base as u64);
        w.u64(self.timer_ctrl as u64);
        w.u64(self.t1khz as u64);
        w.u64(self.tm0_reload as u64);
        w.u64(self.tm1_reload as u64);
        w.u64(self.audio_ch0_ctrl as u64);
        w.u64(self.audio_ch0_data as u64);
        w.u64(self.audio_ch1_ctrl as u64);
        w.u64(self.audio_ch1_data as u64);
        w.u64(self.byte_mirror as u64);
        w.u64(self.nibble_swap as u64);
        w.u64(self.addr_irq as u64);
        w.u64(self.addr_nmi as u64);
        w.u64(self.cycles.to_bits());
        w.u64(self.tm0.to_bits());
        w.u64(self.tm1.to_bits());
        w.u64(self.tm0_counter.to_bits());
        w.u64(self.tm1_counter.to_bits());
        w.u64(self.t1khz_counter.to_bits());
        w.u64(self.audio_at.to_bits());
        w.u64(self.audio_pos.to_bits());
        w.u64(self.audio_acc.to_bits());
        w.u64(self.instructions);
        w.u64(self.interrupts);
        for v in self.ram {
            w.u64(v as u64);
        }
        for v in self.dpram {
            w.u64(v as u64);
        }
        for v in self.pdir {
            w.u64(v as u64);
        }
        for v in self.attr {
            w.u64(v as u64);
        }
        for v in self.platch {
            w.u64(v as u64);
        }
        for v in self.pin0 {
            w.u64(v as u64);
        }
        for v in self.pin1 {
            w.u64(v as u64);
        }
        for v in self.envelope {
            w.u64(v.to_bits());
        }
        for v in self.toggle {
            w.u64(v.to_bits());
        }
        for v in self.dac {
            w.u64(v.to_bits());
        }
        w.bytes(&self.ee.data);
        for v in [
            self.ee.state,
            self.ee.bits,
            self.ee.scl,
            self.ee.sda,
            self.ee.rx,
            self.ee.tx,
        ] {
            w.u8(v);
        }
        w.u16(self.ee.address as u16);
        w.bool(self.ee.ack);
        w.u32(self.pcm.len() as u32);
        for v in &self.pcm {
            w.u16(*v as u16);
        }
        let digest = sha256(&w.0);
        w.bytes(&digest);
        Ok(w.0)
    }
    pub fn restored(&self, data: &[u8]) -> Result<Self> {
        require(
            (15000..=256000).contains(&data.len()),
            "Invalid SPLB32 save size",
        )?;
        let body = &data[..data.len() - 32];
        require(
            sha256(body).as_slice() == &data[data.len() - 32..],
            "Save integrity check failed",
        )?;
        let mut r = Decoder::new(body);
        require(
            &r.array::<8>()? == MAGIC && r.u16()? == 1 && r.u16()? == 6 && r.u8()? == 26,
            "Unsupported SPLB32 save schema or profile",
        )?;
        require(
            r.array::<32>()? == sha256(&self.rom),
            "Save belongs to another firmware",
        )?;
        let mut m = Self::new(&self.rom)?;
        m.pc = r.u64()? as i64;
        m.a = r.u64()? as i64;
        m.x = r.u64()? as i64;
        m.y = r.u64()? as i64;
        m.sp = r.u64()? as i64;
        m.nf = r.u64()? as i64;
        m.vf = r.u64()? as i64;
        m.bf = r.u64()? as i64;
        m.df = r.u64()? as i64;
        m.iflag = r.u64()? as i64;
        m.zf = r.u64()? as i64;
        m.cf = r.u64()? as i64;
        m.rom_bank = r.u64()? as i64;
        m.cpu_enbl = r.u64()? as i64;
        m.cpu_div = r.u64()? as i64;
        m.clk32768_enable = r.u64()? as i64;
        m.lcd_ctrl1 = r.u64()? as i64;
        m.int_ctrl = r.u64()? as i64;
        m.ireq = r.u64()? as i64;
        m.nmi_ctrl = r.u64()? as i64;
        m.nmireq = r.u64()? as i64;
        m.wakeup_ctrl = r.u64()? as i64;
        m.wakeupreq = r.u64()? as i64;
        m.time_base = r.u64()? as i64;
        m.timer_ctrl = r.u64()? as i64;
        m.t1khz = r.u64()? as i64;
        m.tm0_reload = r.u64()? as i64;
        m.tm1_reload = r.u64()? as i64;
        m.audio_ch0_ctrl = r.u64()? as i64;
        m.audio_ch0_data = r.u64()? as i64;
        m.audio_ch1_ctrl = r.u64()? as i64;
        m.audio_ch1_data = r.u64()? as i64;
        m.byte_mirror = r.u64()? as i64;
        m.nibble_swap = r.u64()? as i64;
        m.addr_irq = r.u64()? as i64;
        m.addr_nmi = r.u64()? as i64;
        m.cycles = f64::from_bits(r.u64()?);
        m.tm0 = f64::from_bits(r.u64()?);
        m.tm1 = f64::from_bits(r.u64()?);
        m.tm0_counter = f64::from_bits(r.u64()?);
        m.tm1_counter = f64::from_bits(r.u64()?);
        m.t1khz_counter = f64::from_bits(r.u64()?);
        m.audio_at = f64::from_bits(r.u64()?);
        m.audio_pos = f64::from_bits(r.u64()?);
        m.audio_acc = f64::from_bits(r.u64()?);
        m.instructions = r.u64()?;
        m.interrupts = r.u64()?;
        for v in &mut m.ram {
            *v = r.u64()? as i64;
        }
        for v in &mut m.dpram {
            *v = r.u64()? as i64;
        }
        for v in &mut m.pdir {
            *v = r.u64()? as i64;
        }
        for v in &mut m.attr {
            *v = r.u64()? as i64;
        }
        for v in &mut m.platch {
            *v = r.u64()? as i64;
        }
        for v in &mut m.pin0 {
            *v = r.u64()? as i64;
        }
        for v in &mut m.pin1 {
            *v = r.u64()? as i64;
        }
        for v in &mut m.envelope {
            *v = f64::from_bits(r.u64()?);
        }
        for v in &mut m.toggle {
            *v = f64::from_bits(r.u64()?);
        }
        for v in &mut m.dac {
            *v = f64::from_bits(r.u64()?);
        }
        m.ee.data = r.array()?;
        m.ee.state = r.u8()?;
        m.ee.bits = r.u8()?;
        m.ee.scl = r.u8()?;
        m.ee.sda = r.u8()?;
        m.ee.rx = r.u8()?;
        m.ee.tx = r.u8()?;
        m.ee.address = r.u16()? as usize;
        m.ee.ack = r.bool()?;
        let count = r.u32()?;
        require(count <= 88200, "Oversized SPLB32 audio queue")?;
        for _ in 0..count {
            m.pcm.push(r.u16()? as i16);
        }
        r.finish()?;
        require(
            (0..65536).contains(&m.pc)
                && (0..256).contains(&m.sp)
                && (0..16).contains(&(m.rom_bank >> 15))
                && m.rom_bank & 32767 == 0
                && (0..=6).contains(&m.cpu_div),
            "Invalid CPU/bank state",
        )?;
        for value in [
            m.a,
            m.x,
            m.y,
            m.sp,
            m.lcd_ctrl1,
            m.int_ctrl,
            m.ireq,
            m.nmi_ctrl,
            m.nmireq,
            m.wakeup_ctrl,
            m.wakeupreq,
            m.time_base,
            m.timer_ctrl,
            m.audio_ch0_ctrl,
            m.audio_ch0_data,
            m.audio_ch1_ctrl,
            m.audio_ch1_data,
            m.byte_mirror,
            m.nibble_swap,
        ] {
            require((0..256).contains(&value), "Invalid SPLB32 byte register")?;
        }
        for value in [m.nf, m.vf, m.bf, m.df, m.iflag, m.zf, m.cf, m.cpu_enbl] {
            require((0..=1).contains(&value), "Invalid SPLB32 flag")?;
        }
        for value in m
            .ram
            .iter()
            .chain(m.dpram.iter())
            .chain(m.pdir.iter())
            .chain(m.attr.iter())
            .chain(m.platch.iter())
            .chain(m.pin0.iter())
            .chain(m.pin1.iter())
        {
            require((0..256).contains(value), "Invalid saved RAM/port value")?;
        }
        for value in [
            m.cycles,
            m.tm0,
            m.tm1,
            m.tm0_counter,
            m.tm1_counter,
            m.t1khz_counter,
            m.audio_at,
            m.audio_pos,
            m.audio_acc,
        ]
        .iter()
        .chain(m.envelope.iter())
        .chain(m.toggle.iter())
        .chain(m.dac.iter())
        {
            require(value.is_finite(), "Non-finite SPLB32 saved state")?;
        }
        require(
            m.cycles >= 0.
                && m.cycles < 1e15
                && (0..=128).contains(&m.clk32768_enable)
                && m.clk32768_enable % 128 == 0
                && m.t1khz >= 0
                && m.t1khz as f64 <= m.cycles / (SUB_DIV * 32.) + 1.
                && m.instructions < 1_000_000_000_000_000
                && m.interrupts < 1_000_000_000_000_000
                && m.audio_at >= 0.
                && m.audio_at <= m.cycles
                && m.audio_pos >= 0.
                && m.audio_pos < HZ as f64
                && m.audio_acc.abs() <= HZ as f64 * 2.,
            "Invalid saved clock/audio",
        )?;
        require(
            (0.0..=65535.).contains(&m.tm0)
                && (0.0..=65535.).contains(&m.tm1)
                && (0..65536).contains(&m.tm0_reload)
                && (0..65536).contains(&m.tm1_reload)
                && m.tm0_counter.abs() <= SUB_DIV * 128.
                && m.tm1_counter.abs() <= SUB_DIV * 128.
                && m.t1khz_counter.abs() <= SUB_DIV * 128.,
            "Invalid timer state",
        )?;
        require(
            m.ee.state <= 4
                && m.ee.bits <= 8
                && m.ee.address < 1024
                && m.ee.scl <= 1
                && m.ee.sda <= 2,
            "Invalid EEPROM bus state",
        )?;
        require(
            m.addr_irq == self.addr_irq
                && m.addr_nmi == self.addr_nmi
                && m.envelope.iter().all(|v| (0.0..=1.0).contains(v))
                && m.toggle.iter().all(|v| *v == 1. || *v == -1.)
                && m.dac.iter().all(|v| (-1.0..=1.0).contains(v)),
            "Invalid saved peripheral state",
        )?;
        Ok(m)
    }
}
