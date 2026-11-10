// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

#[test]
fn every_flag_control_mask_decodes_exactly() {
    for mask in 0..16 {
        assert_eq!(
            Instruction::decode(0xF40 | mask),
            Some(Instruction::SetFlags(mask as u8))
        );
        assert_eq!(
            Instruction::decode(0xF50 | mask),
            Some(Instruction::ResetFlags(mask as u8))
        );
    }
}

#[test]
fn flag_control_matches_python_for_every_mask_flags_delay_and_pc_layout() {
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let mut comparisons = 0;
    for row in include_str!("../../../tests/data/e0c6200_flag_reference.txt").lines() {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('|').map(hex).collect();
        assert_eq!(fields.len(), 9);
        let opcode = fields[0];
        let pc = fields[3];
        cpu.program[pc as usize] = opcode;
        cpu.state = CpuState {
            a: 3,
            b: 5,
            ix: 0xF00,
            iy: 0xE50,
            sp: 0xA5,
            pc,
            npc: 0x1A00,
            interrupt_delay: fields[2] != 0,
            ..CpuState::default()
        };
        set_flags(&mut cpu.state, fields[1]);
        cpu.ram = std::array::from_fn(|i| ((i * 3 + 1) & 15) as u8);
        cpu.lcd = std::array::from_fn(|i| ((i + 5) & 15) as u8);
        let mut expected_state = cpu.state.clone();
        set_flags(&mut expected_state, fields[5]);
        expected_state.interrupt_delay = fields[6] != 0;
        expected_state.pc = fields[7];
        expected_state.npc = fields[8];
        let expected_ram = cpu.ram;
        let expected_lcd = cpu.lcd;
        assert_eq!(
            cpu.step(),
            Ok(Step {
                pc,
                opcode,
                cycles: fields[4] as u8,
            }),
            "{row}"
        );
        assert_eq!(cpu.state, expected_state, "{row}");
        assert_eq!(cpu.ram, expected_ram, "{row}");
        assert_eq!(cpu.lcd, expected_lcd, "{row}");
        comparisons += 1;
    }
    assert_eq!(comparisons, 4096);
}

#[test]
fn set_uses_or_and_reset_uses_a_keep_mask_for_each_flag() {
    for (opcode, initial_flags, expected_flags) in [
        (0xF40, 0xA, 0xA),
        (0xF4F, 0, 0xF),
        (0xF50, 0xF, 0),
        (0xF5F, 0xF, 0xF),
        (0xF41, 0xA, 0xB),
        (0xF42, 0x9, 0xB),
        (0xF44, 0x3, 0x7),
        (0xF48, 0x5, 0xD),
        (0xF51, 0xF, 1),
        (0xF52, 0xF, 2),
        (0xF54, 0xF, 4),
        (0xF58, 0xF, 8),
    ] {
        let mut cpu = program(&[opcode]);
        set_flags(&mut cpu.state, initial_flags);
        assert_eq!(cpu.step().unwrap().cycles, 7);
        assert_eq!(flags(&cpu.state), expected_flags, "{opcode:03X}");
        assert_eq!((cpu.state.pc, cpu.state.npc), (0x101, 0x101));
    }
}

#[test]
fn interrupt_delay_tracks_new_enable_transitions_and_expires_without_dispatch() {
    let mut cpu = program(&[0xF48, 0xF48, 0xF57, 0xF48, 0xFFB, 0xE5F, 0xF40, 0xF5F]);
    for (expected_flags, delay, pc, npc) in [
        (8, true, 0x101, 0x101),
        (8, false, 0x102, 0x102),
        (0, false, 0x103, 0x103),
        (8, true, 0x104, 0x104),
        (8, false, 0x105, 0x105),
        (8, true, 0x106, 0x1F00),
        (8, false, 0x107, 0x107),
        (8, false, 0x108, 0x108),
    ] {
        cpu.step().unwrap();
        assert_eq!(flags(&cpu.state), expected_flags);
        assert_eq!(cpu.state.interrupt_delay, delay);
        assert_eq!((cpu.state.pc, cpu.state.npc), (pc, npc));
        assert_eq!(cpu.state.sp, 0);
        assert_eq!(cpu.ram, [0; RAM_NIBBLES]);
    }
    // RST keeps I here but must expire the incoming delay, as clock() does.
    let mut cpu = program(&[0xF5F]);
    set_flags(&mut cpu.state, 15);
    cpu.state.interrupt_delay = true;
    cpu.step().unwrap();
    assert_eq!(flags(&cpu.state), 15);
    assert!(!cpu.state.interrupt_delay);
}

#[test]
fn errors_after_enabling_interrupts_preserve_the_entire_state_and_delay() {
    for opcode in [0xFCC, 0xFDC, 0xFF9, 0xE2F] {
        let mut cpu = program(&[0xF48, opcode]);
        cpu.state.ix = 0xF00;
        cpu.state.iy = 0xE50;
        cpu.state.sp = 0xFF;
        cpu.ram = [9; RAM_NIBBLES];
        cpu.lcd = [7; LCD_NIBBLES];
        cpu.step().unwrap();
        assert!(cpu.state.interrupt_enable);
        assert!(cpu.state.interrupt_delay);
        let before = cpu.clone();
        let error = if opcode == 0xE2F {
            CpuError::UnsupportedAddress(0xF00)
        } else {
            CpuError::UnsupportedOpcode { pc: 0x101, opcode }
        };
        assert_eq!(cpu.step(), Err(error));
        assert_eq!(cpu, before);
    }
}
