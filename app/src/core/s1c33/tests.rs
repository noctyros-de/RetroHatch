// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Synthetic expectations from unchanged C code; no firmware, host UI or ROM downloads.
use super::{
    cpu::{Cpu, Fault, Flags},
    memory::{Bus, BusError, Memory},
    profile::{HardwareProfile, PROFILES},
};
fn hash(mut h: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        h = (h ^ u32::from(b)).wrapping_mul(16777619);
    }
    h
}
fn hash_word(h: u32, value: u32) -> u32 {
    hash(h, &value.to_le_bytes())
}
fn memory(profile: usize) -> Memory {
    let p = PROFILES[profile];
    let mut image = vec![255; p.rom_size as usize];
    let off = (p.ttbr_reset - p.rom_base) as usize;
    image[off..off + 4].copy_from_slice(&0x1000u32.to_le_bytes());
    let mut m = Memory::new(p, &image).unwrap();
    for (i, v) in m.a0ram.iter_mut().enumerate() {
        *v = (i * 37 + 11) as u8;
    }
    for (i, v) in m.ivram.iter_mut().enumerate() {
        *v = (i * 17 + 3) as u8;
    }
    for (i, v) in m.dstram.iter_mut().enumerate() {
        *v = (i * 13 + 7) as u8;
    }
    for i in 0..80 {
        m.a0ram[0x4000 + i * 4..0x4004 + i * 4].copy_from_slice(&0x1800u32.to_le_bytes());
    }
    m.a0ram[0x1800..0x1802].copy_from_slice(&0x6c75u16.to_le_bytes());
    m
}
fn snapshot(c: &Cpu, m: &Memory, reads: u32) -> Vec<u64> {
    let mut values = vec![0];
    values.extend(c.r.map(u64::from));
    values.extend([
        c.sp as u64,
        c.alr as u64,
        c.ahr as u64,
        c.ttbr as u64,
        c.idir as u64,
        c.dbbr as u64,
        c.pc as u64,
        c.flags.pack() as u64,
        c.flags.rest as u64,
        c.ext_count as u64,
        c.ext_values[0] as u64,
        c.ext_values[1] as u64,
        c.halted as u64,
        c.hard_halt as u64,
        c.cycles,
        c.wake_at,
        c.pending_irq as u64,
        c.pending_level as u64,
        c.halt_wakes,
        c.wake_request as u64,
    ]);
    let ram = hash(hash(hash(2166136261, &m.a0ram), &m.ivram), &m.dstram);
    values.push(ram as u64);
    let f = m.flash_state;
    values.extend([
        f.command as u64,
        f.autoselect as u64,
        f.busy_reads as u64,
        f.toggle as u64,
        f.operation_data as u64,
        f.dirty as u64,
        f.programs,
        f.erases,
        m.last_io_read as u64,
        reads as u64,
    ]);
    let bytes = m.flash();
    let flash = hash(
        hash(
            hash(
                hash(2166136261, &bytes[..1024]),
                &bytes[bytes.len() - 1024..],
            ),
            &bytes[0x1234..0x1244],
        ),
        &bytes[0x20000..0x20010],
    );
    values.push(flash as u64);
    let mut display = 2166136261;
    for rgb in m.lcd.rgb(false).chunks_exact(3) {
        display = hash(display, &[rgb[2], rgb[1], rgb[0], 255]);
    }
    values.push(display as u64);
    let l = &m.lcd;
    values.extend([
        l.command as u64,
        l.argc as u64,
        l.xs as u64,
        l.xe as u64,
        l.ys as u64,
        l.ye as u64,
        l.x as u64,
        l.y as u64,
        l.madctl as u64,
        l.colmod as u64,
        l.sleep_out as u64,
        l.display_on as u64,
        l.inverted as u64,
        l.pending_count as u64,
        l.ramwr_bytes,
        l.commands,
        l.frame_marker,
    ]);
    let gram: Vec<u8> = l.gram.iter().flat_map(|p| p.to_le_bytes()).collect();
    values.extend([
        hash(2166136261, &gram) as u64,
        hash(2166136261, &l.args) as u64,
        hash(2166136261, &l.pending_pixel) as u64,
        hash(2166136261, &l.gamma_positive) as u64,
        hash(2166136261, &l.gamma_negative) as u64,
        l.gamma_counts[0] as u64,
        l.gamma_counts[1] as u64,
        l.power_c0[0] as u64,
        l.power_c0[1] as u64,
        l.power_c2 as u64,
        l.vcom_c5 as u64,
        l.vcom_c6 as u64,
        l.power_counts[0] as u64,
        l.power_counts[1] as u64,
        l.power_counts[2] as u64,
        l.power_counts[3] as u64,
    ]);
    values
}

#[test]
fn cpu_memory_nor_and_lcd_match_the_original_c_expectations() {
    let mut count = 0;
    for line in include_str!("../../../tests/data/s1c33_reference.txt")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let (input, expected) = line.split_once(" | ").unwrap();
        let mut parts = input.split_whitespace();
        let name = parts.next().unwrap();
        let v: Vec<u64> = parts.map(|s| u64::from_str_radix(s, 16).unwrap()).collect();
        let expected: Vec<u64> = expected
            .split_whitespace()
            .map(|s| u64::from_str_radix(s, 16).unwrap())
            .collect();
        let mut m = memory(v[0] as usize);
        let mut flags = Flags::default();
        flags.unpack(v[2] as u32);
        let mut cpu = Cpu {
            flags,
            pending_irq: v[3] as u32,
            pending_level: v[4] as u8,
            halted: v[5] != 0,
            hard_halt: v[6] != 0,
            wake_request: v[7] != 0,
            clock_option: v[8] as u32,
            cycles: v[9],
            wake_at: v[10],
            sp: v[27] as u32,
            alr: v[28] as u32,
            ahr: v[29] as u32,
            ttbr: v[30] as u32,
            idir: v[31] as u32,
            dbbr: v[32] as u32,
            pc: v[33] as u32,
            ..Default::default()
        };
        for i in 0..16 {
            cpu.r[i] = v[11 + i] as u32;
        }
        let code = &v[35..35 + v[34] as usize];
        let steps = v[1];
        let mut reads = 2166136261;
        let result = if steps == 0xffff {
            for operation in code.chunks_exact(3) {
                let a = operation[1] as u32;
                let value = operation[2] as u32;
                match operation[0] {
                    0 => reads = hash_word(reads, m.read8(a).unwrap() as u32),
                    1 => reads = hash_word(reads, m.read16(a).unwrap() as u32),
                    2 => reads = hash_word(reads, m.read32(a).unwrap()),
                    3 => m.write8(a, value as u8).unwrap(),
                    4 => m.write16(a, value as u16).unwrap(),
                    5 => m.write32(a, value).unwrap(),
                    _ => unreachable!(),
                }
            }
            Ok(())
        } else {
            for (i, &word) in code.iter().enumerate() {
                m.a0ram[0x1000 + i * 2..0x1002 + i * 2]
                    .copy_from_slice(&(word as u16).to_le_bytes());
            }
            if steps == 0 {
                cpu.reset(&mut m, PROFILES[v[0] as usize].ttbr_reset)
            } else {
                let mut result = Ok(());
                for _ in 0..steps {
                    let before = cpu.clone();
                    let memory_before = snapshot(&cpu, &m, reads);
                    match cpu.step(&mut m) {
                        Ok(_) => {}
                        Err(e) => {
                            assert_eq!(cpu, before, "non-atomic CPU error in {name}");
                            assert_eq!(
                                snapshot(&cpu, &m, reads),
                                memory_before,
                                "non-atomic bus error in {name}"
                            );
                            result = Err(e);
                            break;
                        }
                    }
                }
                result
            }
        };
        if expected[0] != 0 {
            assert!(result.is_err(), "C rejected {name}, Rust accepted it");
        } else {
            result.unwrap_or_else(|e| panic!("Unexpected {name}: {e}"));
            let actual = snapshot(&cpu, &m, reads);
            assert_eq!(
                actual, expected,
                "Reference state differs for {name}: {input}"
            );
        }
        count += 1;
    }
    assert!(count > 1000, "Missing meaningful C oracle coverage");
}

#[test]
fn unsupported_peripherals_and_delay_slot_failures_roll_back_every_effect() {
    let mut m = memory(0);
    let mut cpu = Cpu {
        pc: 0x1000,
        sp: 0x6000,
        ..Default::default()
    };
    // call.d r0, followed by a store into an unimplemented I/O block.
    cpu.r[0] = 0x1200;
    cpu.r[1] = 0x300000;
    cpu.r[2] = 0x55;
    m.a0ram[0x1000..0x1004].copy_from_slice(&[0x00, 0x07, 0x12, 0x34]);
    let before = cpu.clone();
    let data = m.a0ram.clone();
    let error = cpu.step(&mut m).unwrap_err();
    assert!(matches!(
        error.fault,
        Fault::Bus(BusError::PeripheralNotImplemented(0x300000))
    ));
    assert_eq!(cpu, before);
    assert_eq!(m.a0ram, data);
    // A later failure must also undo an IRQ frame already pushed by this step.
    cpu.pending_irq = 20;
    cpu.pending_level = 3;
    cpu.flags.ie = true;
    m.a0ram[0x1800..0x1802].copy_from_slice(&0xffffu16.to_le_bytes());
    let before = cpu.clone();
    let data = m.a0ram.clone();
    assert!(cpu.step(&mut m).is_err());
    assert_eq!(cpu, before);
    assert_eq!(m.a0ram, data);
}

#[test]
fn unsupported_coprocessor_operations_are_explicit_instead_of_reference_noops() {
    for word in [0xb100u16, 0xb500, 0xbf00, 0xbf45, 0xe000] {
        let mut m = memory(0);
        m.a0ram[0x1000..0x1002].copy_from_slice(&word.to_le_bytes());
        let mut cpu = Cpu {
            pc: 0x1000,
            ..Default::default()
        };
        let before = cpu.clone();
        assert!(cpu.step(&mut m).is_err(), "unsupported {word:04x}");
        assert_eq!(cpu, before);
    }
}

#[test]
fn hardware_maps_follow_source_including_four_kib_dstram_and_zero_sleep_address() {
    assert_eq!(PROFILES.len(), 11);
    for (index, profile) in PROFILES.iter().enumerate() {
        assert_eq!(
            profile.dstram_size, 4096,
            "device-map.md incorrectly says 2 KiB"
        );
        let m = memory(index);
        assert!(m.executable(0x84fff));
        assert!(!m.executable(0x85000));
        assert!(Memory::new(*profile, &[0; 16]).is_err());
    }
    let hexa = HardwareProfile::find("plus-color-hexa").unwrap();
    assert!(hexa.has_sleep_flag);
    assert_eq!(hexa.sleep_flag, 0);
    assert!(hexa.flash_top_boot);
    assert_eq!(HardwareProfile::find("4u").unwrap().title, "Tamagotchi 4U+");
    assert_eq!(
        HardwareProfile::find("4u-plain").unwrap().title,
        "Tamagotchi 4U"
    );
}

#[test]
fn memory_transactions_restore_flash_status_lcd_and_ram_without_copying_rom_per_step() {
    let mut m = memory(6);
    let base = m.profile.rom_base;
    let original = m.flash().to_vec();
    let lcd = m.lcd.clone();
    let flash = m.flash_state;
    let ram = m.a0ram.clone();
    m.begin();
    for value in [0xaa, 0x55, 0xa0, 0] {
        m.write8(base, value).unwrap();
    }
    m.read16(base).unwrap();
    m.write8(0x600000, 0x11).unwrap();
    m.write32(0x200, 123).unwrap();
    m.rollback();
    assert_eq!(m.flash(), original);
    assert_eq!(m.flash_state, flash);
    assert_eq!(m.lcd, lcd);
    assert_eq!(m.a0ram, ram);
}

fn synthetic_machine() -> super::machine::Machine {
    let p = PROFILES[0];
    let mut bytes = vec![255; p.rom_size as usize];
    bytes[(p.ttbr_reset - p.rom_base) as usize..(p.ttbr_reset - p.rom_base) as usize + 4]
        .copy_from_slice(&(p.rom_base + 0x1000).to_le_bytes());
    bytes[0x1000..0x1004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
    super::machine::Machine::new(&bytes, p).unwrap()
}
#[test]
fn peripheral_clock_gpio_timers_and_rtc_match_the_reference_rules() {
    use super::peripherals::Peripherals;
    let profile = PROFILES[0];
    let mut p = Peripherals::new(profile);
    p.write32(0x301b08, 4, profile);
    assert_eq!(p.mclk, 18432000.0);
    p.write8(0x301b24, 0x96, profile);
    p.write32(0x301b08, 4, profile);
    assert_eq!(p.mclk, 32768.0);
    p.write8(0x300260, 3, profile);
    p.write8(0x300270, 1, profile);
    p.buttons(1);
    assert_eq!(p.read8(0x300380, profile) & 7, 6);
    assert_eq!((p.pending, p.level), (16, 3));
    p.write8(0x300280, 1, profile);
    assert_eq!(p.pending, 0);
    p.buttons(0);
    assert_eq!(p.read8(0x300380, profile) & 7, 7);
    p.write8(0x300544, 2, profile);
    assert_eq!(p.read32(0x300540, profile) & 0xffff, 0x350);
    for x in 0..6 {
        let b = 0x300780 + x * 8;
        p.write8(b, 2, profile);
        p.write8(b + 2, 4, profile);
        p.write8(b + 6, 3, profile);
        p.write8(0x3007e0 + x * 2, 8, profile);
    }
    p.cycles = 4;
    p.tick(4, profile);
    for timer in &p.timers {
        assert_eq!(timer.counter, 0);
    }
    for reg in 0x300282..=0x300284 {
        assert_eq!(p.read8(reg, profile) & 0xcc, 0xcc);
    }
    p.write8(0x301910, 0x59, profile);
    p.write8(0x301914, 0x59, profile);
    p.write8(0x301918, 0x23, profile);
    p.write8(0x30191c, 0x28, profile);
    p.write8(0x301920, 2, profile);
    p.write8(0x301924, 0x24, profile);
    p.tick(32768, profile);
    assert_eq!(p.read8(0x301910, profile), 0);
    assert_eq!(p.read8(0x30191c, profile), 0x29);
    assert_eq!(p.rtc_seconds, 1);
}
#[test]
fn s1c33_states_restore_full_machine_and_reject_corruption_atomically() {
    let mut m = synthetic_machine();
    for _ in 0..300 {
        m.step().unwrap();
    }
    m.buttons(2);
    let hash = [17; 32];
    let saved = m.save_state(hash).unwrap();
    let mut restored = super::machine::Machine::restored(&saved, hash).unwrap();
    assert_eq!(saved, restored.save_state(hash).unwrap());
    for _ in 0..100 {
        m.step().unwrap();
        restored.step().unwrap();
    }
    assert_eq!(
        m.save_state(hash).unwrap(),
        restored.save_state(hash).unwrap()
    );
    let mut corrupted = saved.clone();
    corrupted[100] ^= 1;
    assert!(super::machine::Machine::restored(&corrupted, hash).is_err());
    assert!(super::machine::Machine::restored(&saved, [18; 32]).is_err());
    assert!(super::machine::Machine::restored(&saved[..80], hash).is_err());
}

#[test]
fn optional_awake_policy_preserves_gram_and_only_overrides_the_reference_sleep_flag() {
    let mut normal = synthetic_machine();
    let mut awake = normal.clone();
    let sleep = normal.memory.profile.sleep_flag as usize;
    normal.memory.a0ram[sleep] = 1;
    awake.memory.a0ram[sleep] = 1;
    for m in [&mut normal, &mut awake] {
        m.memory.lcd.gram[0] = 0xf800;
    }
    assert!(normal.rgb().iter().all(|&v| v == 0));
    awake.keep_screen_awake = true;
    normal.step().unwrap();
    awake.step().unwrap();
    assert_eq!(normal.memory.a0ram[sleep], 1);
    assert_eq!(awake.memory.a0ram[sleep], 0);
    assert_eq!(normal.cpu, awake.cpu);
    assert_eq!(normal.seconds(), awake.seconds());
    assert_eq!(awake.rgb()[..3], [255, 0, 0]);
    assert!(!normal.keep_screen_awake);
    awake.keep_screen_awake = false;
    assert_eq!(awake.rgb(), normal.rgb());
}

#[test]
fn original_id_profile_preserves_gpio_rtc_lcd_and_full_state() {
    use super::{machine::Machine, peripherals::Peripherals};
    let profile = HardwareProfile::find("id").unwrap();
    assert_eq!(
        (profile.rom_base, profile.rom_size, profile.ttbr_reset),
        (0xc00000, 0x400000, 0xc00000)
    );
    assert_eq!(
        (profile.osc3_hz, profile.sleep_flag, profile.ir_ctx_ptr),
        (18_432_000, 0xefc, 0x842bc)
    );
    let mut bytes = vec![255; profile.rom_size as usize];
    bytes[..4].copy_from_slice(&(profile.rom_base + 0x1000).to_le_bytes());
    bytes[0x1000..0x1004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
    let mut machine = Machine::new(&bytes, profile).unwrap();
    let mut peripherals = Peripherals::new(profile);
    peripherals.write8(0x300260, 3, profile);
    peripherals.write8(0x300270, 7, profile);
    for (mask, vector) in [(1, 16), (2, 17), (4, 18)] {
        peripherals.buttons(mask);
        assert_eq!(peripherals.read8(0x300380, profile) & 7, 7 ^ mask);
        assert_eq!(peripherals.pending, vector);
        peripherals.write8(0x300280, mask, profile);
        peripherals.buttons(0);
        assert_eq!(peripherals.read8(0x300380, profile) & 7, 7);
    }
    peripherals.tick(18_432_000, profile);
    assert_eq!(peripherals.rtc_seconds, 1);
    machine.memory.peripherals = Some(peripherals);
    machine.memory.lcd.write_command(0x11);
    machine.memory.lcd.write_command(0x29);
    machine.memory.lcd.gram[0] = 0xf800;
    for _ in 0..300 {
        machine.step().unwrap();
    }
    let hash = crate::core::state::sha256(&bytes);
    let saved = machine.save_state(hash).unwrap();
    assert_eq!(saved[12], 8); // Existing P's states retain code 7.
    let mut restored = Machine::restored(&saved, hash).unwrap();
    assert_eq!(restored.memory.profile, profile);
    assert_eq!(restored.rgb(), machine.rgb());
    assert_eq!(restored.save_state(hash).unwrap(), saved);
    for _ in 0..300 {
        machine.step().unwrap();
        restored.step().unwrap();
    }
    assert_eq!(
        restored.save_state(hash).unwrap(),
        machine.save_state(hash).unwrap()
    );
    let flash = machine.memory.flash().to_vec();
    machine.reset().unwrap();
    assert_eq!(machine.memory.profile, profile);
    assert_eq!(machine.cpu.pc, profile.rom_base + 0x1000);
    assert_eq!(machine.memory.flash(), flash);
    assert!(Machine::new(&vec![255; 0x800000], profile).is_err());
    assert_eq!(
        crate::core::profile::DeviceProfile::parse("idl"),
        Some(crate::core::profile::DeviceProfile::Idl)
    );
    assert_eq!(
        crate::core::profile::DeviceProfile::parse("id-melody"),
        Some(crate::core::profile::DeviceProfile::IdMelody)
    );
}

#[test]
fn id_family_profiles_keep_distinct_save_ids_and_shared_board_behavior() {
    use super::machine::Machine;
    use crate::core::profile::DeviceProfile;
    let family = [
        (DeviceProfile::Id, 8),
        (DeviceProfile::IdMelody, 9),
        (DeviceProfile::Idl, 10),
        (DeviceProfile::IdlEnglish, 11),
        (DeviceProfile::IdlPrincess, 12),
        (DeviceProfile::Idl15, 13),
    ];
    for (device, code) in family {
        let profile = HardwareProfile::find(device.s1c33_key().unwrap()).unwrap();
        assert_eq!(DeviceProfile::parse(device.id()), Some(device));
        assert_eq!(device.rom_bytes(), profile.rom_size as usize);
        assert_eq!(
            (profile.osc3_hz, profile.lcd_cmd_addr, profile.lcd_data_addr),
            (18432000, 0x600000, 0x600001)
        );
        let mut bytes = vec![255; profile.rom_size as usize];
        let reset = (profile.ttbr_reset - profile.rom_base) as usize;
        bytes[reset..reset + 4].copy_from_slice(&(profile.rom_base + 0x1000).to_le_bytes());
        bytes[0x1000..0x1004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
        let mut machine = Machine::new(&bytes, profile).unwrap();
        machine.buttons(1);
        machine.buttons(0);
        machine.memory.lcd.write_command(0x11);
        machine.memory.lcd.write_command(0x29);
        machine.memory.lcd.gram[0] = 0xf800;
        for _ in 0..300 {
            machine.step().unwrap();
        }
        let hash = crate::core::state::sha256(&bytes);
        let state = machine.save_state(hash).unwrap();
        assert_eq!(state[12], code);
        let mut restored = Machine::restored(&state, hash).unwrap();
        assert_eq!(restored.memory.profile, profile);
        assert_eq!(restored.rgb().len(), 128 * 128 * 3);
        assert_eq!(restored.save_state(hash).unwrap(), state);
        for _ in 0..300 {
            machine.step().unwrap();
            restored.step().unwrap();
        }
        assert_eq!(
            restored.save_state(hash).unwrap(),
            machine.save_state(hash).unwrap()
        );
        assert!(Machine::restored(&state, [91; 32]).is_err());
        assert_eq!(device.supports_keep_screen_awake(), profile.has_sleep_flag);
    }
}

#[test]
fn plus_color_boards_preserve_retail_gpio_clock_flash_and_complete_states() {
    use super::{machine::Machine, peripherals::Peripherals};
    use crate::core::{profile::DeviceProfile, state::sha256};
    for (device, code, strap, sleep) in [
        (DeviceProfile::PlusColor, 14, 0, 0x368),
        (DeviceProfile::PlusColorHexa, 15, 0x40, 0),
    ] {
        let profile = HardwareProfile::find(device.s1c33_key().unwrap()).unwrap();
        assert_eq!(DeviceProfile::parse(device.id()), Some(device));
        assert_eq!(profile.osc3_hz, 20_000_000);
        assert_eq!(device.osc2_hz(), 20_000_000);
        assert_eq!(
            (profile.rom_base, profile.ttbr_reset, profile.rom_size),
            (0xc00000, 0xc00000, 0x400000)
        );
        assert!(profile.flash_top_boot && profile.ir_gpio && profile.has_sleep_flag);
        assert_eq!((profile.ir_gpio_tx_bit, profile.ir_gpio_rx_bit), (3, 5));
        assert_eq!(profile.sleep_flag, sleep);
        assert!(!device.has_knock());
        let mut p = Peripherals::new(profile);
        assert_eq!(p.read8(0x300380, profile), 7 | 0x20 | strap);
        p.write8(0x300260, 3, profile);
        p.write8(0x300270, 7, profile);
        for (mask, vector) in [(1, 16), (2, 17), (4, 18)] {
            p.buttons(mask);
            assert_eq!(p.read8(0x300380, profile), (7 ^ mask) | 0x20 | strap);
            assert_eq!(p.pending, vector);
            p.write8(0x300280, mask, profile);
            p.buttons(0);
        }
        p.tick(20_000_000, profile);
        assert_eq!(p.rtc_seconds, 1);
        p.write8(0x301b24, 0x96, profile);
        p.write32(0x301b08, 0x100, profile);
        assert_eq!(p.mclk, 10_000_000.0);
        p.tick(10_000_000, profile);
        assert_eq!(p.rtc_seconds, 2);
        p.write32(0x301b08, 4, profile);
        assert_eq!(p.mclk, 32768.0);
        p.tick(32768, profile);
        assert_eq!(p.rtc_seconds, 3);
        let mut image = vec![255; profile.rom_size as usize];
        image[..4].copy_from_slice(&0xc01000u32.to_le_bytes());
        image[0x1000..0x1004].copy_from_slice(&[0x11, 0x60, 0xff, 0x1e]);
        let mut m = Machine::new(&image, profile).unwrap();
        for value in [0xaa, 0x55, 0xa0] {
            m.memory.write8(profile.rom_base, value).unwrap();
        }
        m.memory.write8(profile.rom_base + 0x3fe010, 0x5a).unwrap();
        assert_eq!(m.memory.flash()[0x3fe010], 0x5a);
        // Complete the real NOR busy/status reads before fetching code from NOR.
        for _ in 0..4 {
            m.memory.read8(profile.rom_base + 0x3fe010).unwrap();
        }
        for _ in 0..100 {
            m.step().unwrap();
        }
        let hash = sha256(&image);
        let state = m.save_state(hash).unwrap();
        assert_eq!(state[12], code);
        let mut restored = Machine::restored(&state, hash).unwrap();
        assert_eq!(restored.save_state(hash).unwrap(), state);
        for _ in 0..100 {
            m.step().unwrap();
            restored.step().unwrap();
        }
        assert_eq!(
            m.save_state(hash).unwrap(),
            restored.save_state(hash).unwrap()
        );
        assert!(Machine::restored(&state, [1; 32]).is_err());
        let flash = m.memory.flash().to_vec();
        m.reset().unwrap();
        assert_eq!(m.cpu.pc, 0xc01000);
        assert_eq!(m.memory.flash(), flash);
        assert_eq!(m.memory.peripherals.as_ref().unwrap().mclk, 20_000_000.0);
    }
}
