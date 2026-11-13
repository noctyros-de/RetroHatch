// SPDX-License-Identifier: GPL-3.0-only
//! Explicit Tama-Go state schema 5. Main firmware remains external and identity-checked.
use super::*;
use crate::core::state::{Decoder, Encoder, MAGIC, sha256};
impl Machine {
    pub fn save_state(&self) -> Vec<u8> {
        let mut w = Encoder(Vec::new());
        w.bytes(MAGIC);
        w.u16(1);
        w.u16(5);
        w.u8(25);
        w.bytes(&sha256(&self.rom));
        w.u16(self.cpu.pc);
        for v in [
            self.cpu.a, self.cpu.x, self.cpu.y, self.cpu.s, self.cpu.p, self.bank, self.keys,
        ] {
            w.u8(v);
        }
        w.bytes(&self.ram);
        w.bytes(&self.dram);
        w.bytes(&self.reg);
        for v in [
            self.cycles,
            self.instructions,
            self.interrupts,
            self.video,
            self.tbh,
            self.timer_due[0],
            self.timer_due[1],
            self.timer_period[0],
            self.timer_period[1],
            self.spi_reads,
            self.audio_at,
            self.audio_pos,
            self.audio_acc as u64,
        ] {
            w.u64(v);
        }
        w.u64(self.figure_ready);
        w.u64(self.key_active);
        w.bool(self.halted);
        w.bool(self.spi_cs);
        w.u32(self.spi_pos);
        w.u32(self.spi_addr);
        w.u8(self.spi_cmd);
        w.bytes(&self.spi_fifo);
        w.u8(self.spi_w);
        w.u8(self.spi_r);
        w.bytes(&self.ee.data);
        for v in [
            self.ee.state,
            self.ee.output,
            self.ee.clk,
            self.ee.line,
            self.ee.shift,
            self.ee.bits,
            self.ee.addressbyte,
        ] {
            w.u8(v);
        }
        w.u16(self.ee.address);
        w.i32(self.dac);
        // Pending PCM is captured too: save/load must not move the sample cursor.
        w.u32(self.pcm.len() as u32);
        for v in &self.pcm {
            w.u16(*v as u16);
        }
        w.bool(self.figure.is_some());
        if let Some(f) = &self.figure {
            w.bytes(&f.hash);
            w.u32(f.bytes.len() as u32);
            w.bytes(&f.bytes);
        }
        let hash = sha256(&w.0);
        w.bytes(&hash);
        w.0
    }
    pub fn restored(&self, data: &[u8]) -> Result<Self, String> {
        require(
            (5000..=8 * 1024 * 1024).contains(&data.len()),
            "Invalid Tama-Go state size",
        )?;
        let body = &data[..data.len() - 32];
        require(
            sha256(body).as_slice() == &data[data.len() - 32..],
            "Save integrity check failed",
        )?;
        let mut r = Decoder::new(body);
        require(
            &r.array::<8>()? == MAGIC && r.u16()? == 1 && r.u16()? == 5 && r.u8()? == 25,
            "Unsupported Tama-Go save schema",
        )?;
        require(
            r.array::<32>()? == sha256(&self.rom),
            "Save belongs to a different firmware",
        )?;
        let mut m = Self::new(&self.rom)?;
        m.cpu.pc = r.u16()?;
        m.cpu.a = r.u8()?;
        m.cpu.x = r.u8()?;
        m.cpu.y = r.u8()?;
        m.cpu.s = r.u8()?;
        m.cpu.p = r.u8()?;
        m.bank = r.u8()?;
        m.keys = r.u8()?;
        m.ram = r.array()?;
        m.dram = r.array()?;
        m.reg = r.array()?;
        m.cycles = r.u64()?;
        m.instructions = r.u64()?;
        m.interrupts = r.u64()?;
        m.video = r.u64()?;
        m.tbh = r.u64()?;
        for v in &mut m.timer_due {
            *v = r.u64()?;
        }
        for v in &mut m.timer_period {
            *v = r.u64()?;
        }
        m.spi_reads = r.u64()?;
        m.audio_at = r.u64()?;
        m.audio_pos = r.u64()?;
        m.audio_acc = r.u64()? as i64;
        m.figure_ready = r.u64()?;
        m.key_active = r.u64()?;
        m.halted = r.bool()?;
        m.spi_cs = r.bool()?;
        m.spi_pos = r.u32()?;
        m.spi_addr = r.u32()?;
        m.spi_cmd = r.u8()?;
        m.spi_fifo = r.array()?;
        m.spi_w = r.u8()?;
        m.spi_r = r.u8()?;
        m.ee.data = r.array()?;
        m.ee.state = r.u8()?;
        m.ee.output = r.u8()?;
        m.ee.clk = r.u8()?;
        m.ee.line = r.u8()?;
        m.ee.shift = r.u8()?;
        m.ee.bits = r.u8()?;
        m.ee.addressbyte = r.u8()?;
        m.ee.address = r.u16()?;
        m.dac = r.i32()?;
        let count = r.u32()? as usize;
        require(count <= 44100 * 2, "Oversized saved audio queue")?;
        for _ in 0..count {
            m.pcm.push(r.u16()? as i16);
        }
        if r.bool()? {
            let hash = r.array()?;
            let len = r.u32()? as usize;
            require(
                len <= 4194304 && r.at.checked_add(len).is_some_and(|end| end <= body.len()),
                "Invalid saved accessory length",
            )?;
            let mut f = figure::Figure::decode(&body[r.at..r.at + len])?;
            r.at += len;
            f.hash = hash;
            m.figure = Some(Arc::new(f));
        }
        r.finish()?;
        require(
            m.bank < 20
                && m.keys <= 15
                && m.ee.state <= 4
                && m.ee.bits < 9
                && m.ee.output <= 1
                && m.ee.clk <= 1
                && m.ee.line <= 1
                && m.ee.address < 4096
                && m.ee.addressbyte <= 2,
            "Invalid Tama-Go peripheral state",
        )?;
        require(
            m.cycles < u64::MAX / 65536 - HZ * 3
                && m.figure_ready <= m.cycles + HZ * 5 / 2
                && m.key_active <= m.cycles + HZ
                && m.video > m.cycles
                && m.video - m.cycles <= HZ / 60
                && m.tbh > m.cycles
                && m.tbh - m.cycles <= HZ / 2
                && m.audio_at == m.cycles * 44100
                && m.audio_pos < HZ
                && m.audio_acc.unsigned_abs() <= 128 * HZ
                && (-128..=127).contains(&m.dac),
            "Invalid saved Tama-Go clock/audio state",
        )?;
        for t in 0..2 {
            require(
                m.timer_period[t] <= ((65536 * HZ) << 16) / 32768
                    && (m.timer_period[t] == 0
                        || (m.timer_due[t] > m.cycles << 16
                            && m.timer_due[t] - (m.cycles << 16) <= m.timer_period[t])),
                "Invalid saved timer",
            )?;
        }
        Ok(m)
    }
}
