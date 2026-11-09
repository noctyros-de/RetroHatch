use super::*;

const TARGETS: [IndexNibble; 4] = [
    IndexNibble::Xh,
    IndexNibble::Xl,
    IndexNibble::Yh,
    IndexNibble::Yl,
];

#[test]
fn every_index_opcode_decodes_target_and_immediate() {
    for (target, field) in TARGETS.into_iter().enumerate() {
        for immediate in 0..16 {
            let offset = ((target as u16) << 4) | immediate;
            assert_eq!(
                Instruction::decode(0xA00 | offset),
                Some(Instruction::AdcIndex(field, immediate as u8))
            );
            assert_eq!(
                Instruction::decode(0xA40 | offset),
                Some(Instruction::CompareIndex(field, immediate as u8))
            );
        }
    }
}

#[test]
fn index_instructions_match_python_for_every_nibble_immediate_and_flags() {
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let expected_ram = std::array::from_fn(|i| ((i * 3 + 1) & 15) as u8);
    let expected_lcd = std::array::from_fn(|i| ((i + 5) & 15) as u8);
    cpu.ram = expected_ram;
    cpu.lcd = expected_lcd;
    let mut comparisons = [0; 8];
    for row in include_str!("../../../tests/data/e0c6200_index_alu_reference.txt").lines() {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('|').collect();
        let base = hex(fields[0]);
        let initial_flags = hex(fields[1]);
        let ix = hex(fields[2]);
        let iy = hex(fields[3]);
        let cycles = hex(fields[4]) as u8;
        let next_pc = hex(fields[5]);
        let next_npc = hex(fields[6]);
        let delay = hex(fields[7]) != 0;
        assert_eq!(fields[8].len(), 64);
        for immediate in 0..16 {
            let pc = [0x100, 0xFFF, 0x1FFF, 0x10FF][(initial_flags & 3) as usize];
            let opcode = base | immediate;
            cpu.program[pc as usize] = opcode;
            cpu.state = CpuState {
                a: 3,
                b: 5,
                ix,
                iy,
                sp: 0xA5,
                pc,
                npc: 0x1A00,
                interrupt_delay: true,
                ..CpuState::default()
            };
            set_flags(&mut cpu.state, initial_flags);
            let offset = immediate as usize * 4;
            let expected_index = hex(&fields[8][offset..offset + 3]);
            let expected_flags = hex(&fields[8][offset + 3..offset + 4]);
            let mut expected_state = cpu.state.clone();
            if (base >> 4) & 3 < 2 {
                expected_state.ix = expected_index;
            } else {
                expected_state.iy = expected_index;
            }
            set_flags(&mut expected_state, expected_flags);
            expected_state.pc = next_pc;
            expected_state.npc = next_npc;
            expected_state.interrupt_delay = delay;
            let context =
                format!("{opcode:03X}, IX={ix:03X}, IY={iy:03X}, flags={initial_flags:X}");
            assert_eq!(cpu.step(), Ok(Step { pc, opcode, cycles }), "{context}");
            assert_eq!(cpu.state, expected_state, "{context}");
            assert_eq!(cpu.ram, expected_ram, "{context}");
            assert_eq!(cpu.lcd, expected_lcd, "{context}");
            comparisons[((base - 0xA00) >> 4) as usize] += 1;
        }
    }
    assert_eq!(comparisons, [32768; 8]);
}

#[test]
fn index_overflows_do_not_carry_into_neighbor_or_page_and_retarget_memory() {
    for (opcode, is_x, expected_address) in [
        (0xA01, true, 0x20F),
        (0xA11, true, 0x2F0),
        (0xA21, false, 0x20F),
        (0xA31, false, 0x2F0),
    ] {
        let store = if is_x { 0xE2A } else { 0xE3A };
        let mut cpu = program(&[opcode, store]);
        cpu.state.ix = if is_x { 0x2FF } else { 1 };
        cpu.state.iy = if is_x { 1 } else { 0x2FF };
        set_flags(&mut cpu.state, 0xC); // D=1/I=1, C=0.
        cpu.write_memory(0x2FF, 5).unwrap();
        cpu.write_memory(1, 7).unwrap();
        cpu.step().unwrap();
        assert_eq!(
            if is_x { cpu.state.ix } else { cpu.state.iy },
            expected_address
        );
        assert_eq!(if is_x { cpu.state.iy } else { cpu.state.ix }, 1);
        assert_eq!(flags(&cpu.state), 0xF); // Zero tests the nibble, not the address.
        assert_eq!(cpu.read_memory(expected_address), Ok(0)); // ADC itself does not write memory.
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory(expected_address), Ok(0xA));
        assert_eq!(cpu.read_memory(0x2FF), Ok(5));
        assert_eq!(cpu.read_memory(1), Ok(7));
        assert_eq!(flags(&cpu.state), 0xF);
    }
}

#[test]
fn index_adc_is_binary_and_index_compare_never_writes() {
    for (adc, compare, is_x, before, after) in [
        (0xA01, 0xA4A, true, 0x29B, 0x2AB),
        (0xA11, 0xA5A, true, 0x2F9, 0x2FA),
        (0xA21, 0xA6A, false, 0x29B, 0x2AB),
        (0xA31, 0xA7A, false, 0x2F9, 0x2FA),
    ] {
        let mut cpu = program(&[adc, compare]);
        if is_x {
            cpu.state.ix = before;
        } else {
            cpu.state.iy = before;
        }
        set_flags(&mut cpu.state, 0xC);
        cpu.step().unwrap();
        assert_eq!(if is_x { cpu.state.ix } else { cpu.state.iy }, after);
        assert_eq!(flags(&cpu.state), 0xC); // 9+1 = A, no decimal correction.
        cpu.step().unwrap();
        assert_eq!(if is_x { cpu.state.ix } else { cpu.state.iy }, after);
        assert_eq!(flags(&cpu.state), 0xE); // Equal: Z=1, C=0; preserve D/I.
    }
}

#[test]
fn arithmetic_may_form_unmapped_addresses_but_followup_access_fails_atomically() {
    for (opcode, access, is_x, before, after) in [
        (0xA01, 0xEC2, true, 0xE4F, 0xE5F),  // LCD gap; read M(X).
        (0xA21, 0xEC7, false, 0xECF, 0xEDF), // Beyond the LCD window; read M(Y).
        (0xA01, 0xE2A, true, 0xF00, 0xF10),  // I/O remains unimplemented.
        (0xA21, 0xE3A, false, 0x330, 0x340), // Unmapped RAM region.
    ] {
        let mut cpu = program(&[opcode, access]);
        if is_x {
            cpu.state.ix = before;
        } else {
            cpu.state.iy = before;
        }
        cpu.write_memory(0, 9).unwrap();
        cpu.write_memory(0xE80, 7).unwrap();
        cpu.step().unwrap(); // No implicit dereferencing of M(X)/M(Y).
        assert_eq!(if is_x { cpu.state.ix } else { cpu.state.iy }, after);
        let before_failed_access = cpu.clone();
        assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(after)));
        assert_eq!(cpu, before_failed_access);
    }
}

#[test]
fn neighboring_index_related_groups_remain_explicitly_unsupported() {
    for opcode in [0x900, 0xAF0, 0xE80, 0xE90, 0xF20, 0xF27, 0xF30, 0xF37] {
        let mut cpu = program(&[opcode]);
        cpu.state.ix = 0xE4F;
        cpu.state.iy = 0xECF;
        cpu.state.npc = 0x1A00;
        cpu.state.interrupt_delay = true;
        set_flags(&mut cpu.state, 15);
        let before = cpu.clone();
        assert_eq!(
            cpu.step(),
            Err(CpuError::UnsupportedOpcode { pc: 0x100, opcode })
        );
        assert_eq!(cpu, before);
    }
}
