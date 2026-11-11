// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Explicit versioned S1C33 states. No native structure dumps or changes to classic/ARM schemas.
use super::{machine::Machine, profile::HardwareProfile};
use crate::core::state::{Decoder, Encoder, MAGIC, require, sha256};
fn real(r: &mut Decoder<'_>) -> Result<f64, String> {
    let value = f64::from_bits(r.u64()?);
    require(value.is_finite(), "Invalid S1C33 floating-point state")?;
    Ok(value)
}
fn vector(r: &mut Decoder<'_>, length: usize) -> Result<Vec<u8>, String> {
    let end =
        r.at.checked_add(length)
            .ok_or("Invalid S1C33 state length")?;
    let bytes = r
        .data
        .get(r.at..end)
        .ok_or("Truncated S1C33 state")?
        .to_vec();
    r.at = end;
    Ok(bytes)
}
impl Machine {
    pub fn save_state(&self, hash: [u8; 32]) -> Result<Vec<u8>, String> {
        let mut e = Encoder(vec![]);
        let c = &self.cpu;
        for value in c.r {
            e.u32(value);
        }
        e.u32(c.flags.pack());
        e.u32(c.flags.rest);
        for value in c.ext_values {
            e.u32(value);
        }
        e.u32(c.sp);
        e.u32(c.alr);
        e.u32(c.ahr);
        e.u32(c.ttbr);
        e.u32(c.idir);
        e.u32(c.dbbr);
        e.u32(c.pc);
        e.u32(c.pending_irq);
        e.u32(c.clock_option);
        e.u8(c.ext_count);
        e.u8(c.pending_level);
        e.u64(c.wake_at);
        e.u64(c.cycles);
        e.u64(c.halt_wakes);
        e.bool(c.halted);
        e.bool(c.hard_halt);
        e.bool(c.wake_request);
        e.u64(self.last_tick);
        e.u64(self.instructions);
        e.u64(self.interrupts);
        let m = &self.memory;
        e.bytes(m.flash());
        e.bytes(&m.a0ram);
        e.bytes(&m.ivram);
        e.bytes(&m.dstram);
        e.u32(m.last_io_read);
        let f = &m.flash_state;
        e.u8(f.command);
        e.u8(f.busy_reads);
        e.u16(f.operation_data);
        e.bool(f.autoselect);
        e.bool(f.toggle);
        e.bool(f.dirty);
        e.u64(f.programs);
        e.u64(f.erases);
        let l = &m.lcd;
        for &pixel in &l.gram {
            e.u16(pixel);
        }
        e.bytes(&l.args);
        e.bytes(&l.gamma_positive);
        e.bytes(&l.gamma_negative);
        e.bytes(&l.gamma_counts);
        e.bytes(&l.power_c0);
        e.bytes(&l.power_counts);
        e.bytes(&l.pending_pixel);
        e.u8(l.pending_count as u8);
        e.u32(l.argc);
        e.u32(l.xs);
        e.u32(l.xe);
        e.u32(l.ys);
        e.u32(l.ye);
        e.u32(l.x);
        e.u32(l.y);
        e.u8(l.command);
        e.u8(l.madctl);
        e.u8(l.colmod);
        e.u8(l.power_c2);
        e.u8(l.vcom_c5);
        e.u8(l.vcom_c6);
        e.bool(l.sleep_out);
        e.bool(l.display_on);
        e.bool(l.inverted);
        e.u64(l.ramwr_bytes);
        e.u64(l.commands);
        e.u64(l.frame_marker);
        let p = m
            .peripherals
            .as_ref()
            .ok_or("S1C33 peripherals are missing")?;
        e.bytes(&p.io);
        for t in &p.timers {
            e.u16(t.counter);
            e.u32(t.accum);
            e.bool(t.a_fired);
        }
        e.u32(p.sccr);
        e.u32(p.pllc);
        e.u32(p.opt);
        e.u32(p.pending);
        e.u8(p.level);
        e.u8(p.buttons);
        e.u64(p.cycles);
        e.u64(p.rtc_seconds);
        e.u64(p.ir_ready_at);
        e.u64(p.irq_count);
        e.u64(p.scan_serial);
        e.u64(p.tone_events);
        e.u64(p.t2_attacks);
        e.bool(p.unlocked);
        e.bool(p.wake);
        e.bool(p.tone_on);
        e.u64(p.mclk.to_bits());
        e.u64(p.osc3.to_bits());
        e.u64(p.seconds.to_bits());
        e.u64(p.rtc_fraction.to_bits());
        e.u64(p.rtc_irq_fraction.to_bits());
        e.u64(p.tone_frequency.to_bits());
        e.u64(p.phase.to_bits());
        e.u64(p.sample_fraction.to_bits());
        e.u64(p.t2_frequency.to_bits());
        e.u64(p.t2_duty.to_bits());
        require(
            p.pcm.len() <= 88200,
            "S1C33 audio queue exceeds the snapshot limit",
        )?;
        e.u32(p.pcm.len() as u32);
        for &sample in &p.pcm {
            e.u16(sample as u16);
        }
        let mut out = Encoder(vec![]);
        out.bytes(MAGIC);
        out.u16(1);
        out.u16(3);
        out.u8(match self.memory.profile.key {
            "ps" => 7,
            "id" => 8,
            "id-melody" => 9,
            "idl" => 10,
            "idl-en" => 11,
            "idl-princess" => 12,
            "idl-15" => 13,
            "plus-color" => 14,
            "plus-color-hexa" => 15,

            _ => return Err("Save support is unavailable for this S1C33 hardware profile".into()),
        });
        out.bytes(&hash);
        out.u32(e.0.len() as u32);
        out.bytes(&e.0);
        let digest = sha256(&out.0);
        out.bytes(&digest);
        Ok(out.0)
    }
    pub fn restored(data: &[u8], hash: [u8; 32]) -> Result<Self, String> {
        require(
            data.len() >= 81 && data.len() <= 64 * 1024 * 1024,
            "Invalid S1C33 state size",
        )?;
        let body = &data[..data.len() - 32];
        require(
            sha256(body).as_slice() == &data[data.len() - 32..],
            "Save integrity check failed",
        )?;
        let mut r = Decoder::new(body);
        require(
            &r.array::<8>()? == MAGIC && r.u16()? == 1 && r.u16()? == 3,
            "Unsupported S1C33 save format or profile",
        )?;
        let profile = match r.u8()? {
            7 => HardwareProfile::find("ps").unwrap(),
            8 => HardwareProfile::find("id").unwrap(),
            9 => HardwareProfile::find("id-melody").unwrap(),
            10 => HardwareProfile::find("idl").unwrap(),
            11 => HardwareProfile::find("idl-en").unwrap(),
            12 => HardwareProfile::find("idl-princess").unwrap(),
            13 => HardwareProfile::find("idl-15").unwrap(),
            14 => HardwareProfile::find("plus-color").unwrap(),
            15 => HardwareProfile::find("plus-color-hexa").unwrap(),

            _ => return Err("Unsupported S1C33 save profile".into()),
        };
        require(r.array::<32>()? == hash, "Save belongs to a different ROM")?;
        require(
            r.u32()? as usize == body.len() - r.at,
            "Invalid S1C33 payload length",
        )?;
        let mut cpu = super::cpu::Cpu::default();
        for v in &mut cpu.r {
            *v = r.u32()?;
        }
        cpu.flags.unpack(r.u32()?);
        cpu.flags.rest = r.u32()?;
        for v in &mut cpu.ext_values {
            *v = r.u32()?;
        }
        cpu.sp = r.u32()?;
        cpu.alr = r.u32()?;
        cpu.ahr = r.u32()?;
        cpu.ttbr = r.u32()?;
        cpu.idir = r.u32()?;
        cpu.dbbr = r.u32()?;
        cpu.pc = r.u32()?;
        cpu.pending_irq = r.u32()?;
        cpu.clock_option = r.u32()?;
        cpu.ext_count = r.u8()?;
        cpu.pending_level = r.u8()?;
        cpu.wake_at = r.u64()?;
        cpu.cycles = r.u64()?;
        cpu.halt_wakes = r.u64()?;
        cpu.halted = r.bool()?;
        cpu.hard_halt = r.bool()?;
        cpu.wake_request = r.bool()?;
        let last_tick = r.u64()?;
        let instructions = r.u64()?;
        let interrupts = r.u64()?;
        let image = vector(&mut r, profile.rom_size as usize)?;
        let mut machine = Self::new(&image, profile)?;
        let m = &mut machine.memory;
        m.a0ram = vector(&mut r, profile.a0ram_size as usize)?;
        m.ivram = vector(&mut r, profile.ivram_size as usize)?;
        m.dstram = vector(&mut r, profile.dstram_size as usize)?;
        m.last_io_read = r.u32()?;
        let f = &mut m.flash_state;
        f.command = r.u8()?;
        f.busy_reads = r.u8()?;
        f.operation_data = r.u16()?;
        f.autoselect = r.bool()?;
        f.toggle = r.bool()?;
        f.dirty = r.bool()?;
        f.programs = r.u64()?;
        f.erases = r.u64()?;
        let l = &mut m.lcd;
        for pixel in &mut l.gram {
            *pixel = r.u16()?;
        }
        l.args = r.array()?;
        l.gamma_positive = r.array()?;
        l.gamma_negative = r.array()?;
        l.gamma_counts = r.array()?;
        l.power_c0 = r.array()?;
        l.power_counts = r.array()?;
        l.pending_pixel = r.array()?;
        l.pending_count = r.u8()? as usize;
        l.argc = r.u32()?;
        l.xs = r.u32()?;
        l.xe = r.u32()?;
        l.ys = r.u32()?;
        l.ye = r.u32()?;
        l.x = r.u32()?;
        l.y = r.u32()?;
        l.command = r.u8()?;
        l.madctl = r.u8()?;
        l.colmod = r.u8()?;
        l.power_c2 = r.u8()?;
        l.vcom_c5 = r.u8()?;
        l.vcom_c6 = r.u8()?;
        l.sleep_out = r.bool()?;
        l.display_on = r.bool()?;
        l.inverted = r.bool()?;
        l.ramwr_bytes = r.u64()?;
        l.commands = r.u64()?;
        l.frame_marker = r.u64()?;
        require(
            l.pending_count < if l.colmod & 15 == 5 { 2 } else { 3 }
                && [l.xs, l.xe, l.ys, l.ye, l.x, l.y]
                    .iter()
                    .all(|&v| v <= 65536),
            "Invalid S1C33 LCD state",
        )?;
        let p = m.peripherals.as_mut().unwrap();
        p.io = vector(&mut r, 8192)?;
        for t in &mut p.timers {
            t.counter = r.u16()?;
            t.accum = r.u32()?;
            t.a_fired = r.bool()?;
        }
        p.sccr = r.u32()?;
        p.pllc = r.u32()?;
        p.opt = r.u32()?;
        p.pending = r.u32()?;
        p.level = r.u8()?;
        p.buttons = r.u8()?;
        p.cycles = r.u64()?;
        p.rtc_seconds = r.u64()?;
        p.ir_ready_at = r.u64()?;
        p.irq_count = r.u64()?;
        p.scan_serial = r.u64()?;
        p.tone_events = r.u64()?;
        p.t2_attacks = r.u64()?;
        p.unlocked = r.bool()?;
        p.wake = r.bool()?;
        p.tone_on = r.bool()?;
        p.mclk = real(&mut r)?;
        p.osc3 = real(&mut r)?;
        p.seconds = real(&mut r)?;
        p.rtc_fraction = real(&mut r)?;
        p.rtc_irq_fraction = real(&mut r)?;
        p.tone_frequency = real(&mut r)?;
        p.phase = real(&mut r)?;
        p.sample_fraction = real(&mut r)?;
        p.t2_frequency = real(&mut r)?;
        p.t2_duty = real(&mut r)?;
        let count = r.u32()? as usize;
        require(count <= 88200, "Invalid S1C33 audio queue")?;
        for _ in 0..count {
            p.pcm.push(r.u16()? as i16);
        }
        r.finish()?;
        require(
            cpu.ext_count <= 2
                && cpu.pending_level <= 7
                && p.level <= 7
                && p.pending <= 88
                && p.buttons <= 7
                && last_tick <= cpu.cycles
                && cpu.cycles - last_tick < 256
                && p.cycles == cpu.cycles,
            "Invalid S1C33 CPU or interrupt state",
        )?;
        require(
            p.osc3 == profile.osc3_hz as f64
                && (1.0..=1e9).contains(&p.mclk)
                && p.seconds >= 0.0
                && (0.0..1.0).contains(&p.rtc_fraction)
                && p.rtc_irq_fraction >= 0.0
                && (0.0..1.0).contains(&p.phase)
                && (0.0..1.0).contains(&p.sample_fraction)
                && (0.0..=1.0).contains(&p.t2_duty),
            "Invalid S1C33 clock or audio state",
        )?;
        machine.cpu = cpu;
        machine.last_tick = last_tick;
        machine.instructions = instructions;
        machine.interrupts = interrupts;
        Ok(machine)
    }
}
