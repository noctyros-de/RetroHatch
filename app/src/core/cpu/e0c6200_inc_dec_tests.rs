use super::*;

#[test]
fn all_increment_decrement_variants_decode_exactly() {
    for address in 0..16 {
        assert_eq!(
            Instruction::decode(0xF60 | address),
            Some(Instruction::IncrementMemory(address as u8))
        );
        assert_eq!(
            Instruction::decode(0xF70 | address),
            Some(Instruction::DecrementMemory(address as u8))
        );
    }
    assert_eq!(
        Instruction::decode(0xFCB),
        Some(Instruction::DecrementStackPointer)
    );
    assert_eq!(
        Instruction::decode(0xFDB),
        Some(Instruction::IncrementStackPointer)
    );
}

#[test]
fn inc_dec_matches_python_for_all_addresses_values_stack_pointers_and_flags() {
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let mut comparisons = [0; 2];
    for row in include_str!("../../../tests/data/e0c6200_inc_dec_reference.txt").lines() {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('|').collect();
        let opcode = hex(fields[0]);
        let initial_flags = hex(fields[1]);
        let cycles = hex(fields[2]) as u8;
        let next_pc = hex(fields[3]);
        let next_npc = hex(fields[4]);
        let delay = hex(fields[5]) != 0;
        let is_sp = matches!(opcode, 0xFCB | 0xFDB);
        let values = if is_sp { 256 } else { 16 };
        assert_eq!(fields[6].len(), values * 3);
        for value in 0..values {
            let pc = [0x100, 0xFFF, 0x1FFF, 0x10FF][(initial_flags & 3) as usize];
            cpu.program[pc as usize] = opcode;
            cpu.state = CpuState {
                a: 3,
                b: 5,
                ix: 0xF00,
                iy: 0xE50,
                sp: if is_sp { value as u8 } else { 0xA5 },
                pc,
                npc: 0x1A00,
                interrupt_delay: true,
                ..CpuState::default()
            };
            set_flags(&mut cpu.state, initial_flags);
            cpu.ram = std::array::from_fn(|i| ((i * 3 + 1) & 15) as u8);
            cpu.lcd = std::array::from_fn(|i| ((i + 5) & 15) as u8);
            if !is_sp {
                cpu.write_memory(opcode & 15, value as u8).unwrap();
            }
            let offset = value * 3;
            let result = hex(&fields[6][offset..offset + 2]) as u8;
            let expected_flags = hex(&fields[6][offset + 2..offset + 3]);
            let mut expected_state = cpu.state.clone();
            let mut expected_ram = cpu.ram;
            let expected_lcd = cpu.lcd;
            if is_sp {
                expected_state.sp = result;
            } else {
                expected_ram[(opcode & 15) as usize] = result;
            }
            set_flags(&mut expected_state, expected_flags);
            expected_state.pc = next_pc;
            expected_state.npc = next_npc;
            expected_state.interrupt_delay = delay;
            let context = format!("{opcode:03X}, value={value:02X}, flags={initial_flags:X}");
            assert_eq!(cpu.step(), Ok(Step { pc, opcode, cycles }), "{context}");
            assert_eq!(cpu.state, expected_state, "{context}");
            assert_eq!(cpu.ram, expected_ram, "{context}");
            assert_eq!(cpu.lcd, expected_lcd, "{context}");
            comparisons[usize::from(is_sp)] += 1;
        }
    }
    assert_eq!(comparisons, [8192, 8192]);
}

#[test]
fn direct_memory_wraps_at_one_nibble_and_ignores_indices_carry_and_decimal() {
    for address in [0, 15] {
        for (base, value, initial_flags, result, expected_flags) in [
            (0xF60, 15, 0xC, 0, 0xF),
            (0xF60, 0, 0xF, 1, 0xC),
            (0xF70, 0, 0xF, 15, 0xD),
            (0xF70, 1, 0xF, 0, 0xE),
            (0xF70, 15, 0xF, 14, 0xC),
        ] {
            let mut cpu = program(&[base | address]);
            cpu.state.ix = 0xF00;
            cpu.state.iy = 0xE50;
            cpu.state.sp = 0xFF;
            set_flags(&mut cpu.state, initial_flags);
            cpu.write_memory(address, value).unwrap();
            cpu.write_memory(0x10, 9).unwrap();
            cpu.write_memory(0xFF, 7).unwrap();
            assert_eq!(cpu.step().unwrap().cycles, 7);
            assert_eq!(cpu.read_memory(address), Ok(result));
            assert_eq!(flags(&cpu.state), expected_flags);
            assert_eq!(cpu.read_memory(0x10), Ok(9));
            assert_eq!(cpu.read_memory(0xFF), Ok(7));
            assert_eq!(
                (cpu.state.ix, cpu.state.iy, cpu.state.sp),
                (0xF00, 0xE50, 0xFF)
            );
        }
    }
}

#[test]
fn stack_pointer_wraps_without_memory_access_or_flag_changes() {
    for (opcode, initial, expected) in [
        (0xFCB, 0, 0xFF),
        (0xFCB, 1, 0),
        (0xFCB, 0xFF, 0xFE),
        (0xFDB, 0xFF, 0),
        (0xFDB, 0xFE, 0xFF),
        (0xFDB, 0, 1),
    ] {
        for initial_flags in [0, 15] {
            let mut cpu = program(&[opcode]);
            cpu.state.sp = initial;
            cpu.state.ix = 0xF00;
            cpu.state.iy = 0xE50;
            set_flags(&mut cpu.state, initial_flags);
            cpu.ram = [9; RAM_NIBBLES];
            cpu.lcd = [7; LCD_NIBBLES];
            assert_eq!(cpu.step().unwrap().cycles, 5);
            assert_eq!(cpu.state.sp, expected);
            assert_eq!(flags(&cpu.state), initial_flags as u8);
            assert_eq!(cpu.ram, [9; RAM_NIBBLES]);
            assert_eq!(cpu.lcd, [7; LCD_NIBBLES]);
        }
    }
}

#[test]
fn neighboring_flag_load_and_stack_opcodes_remain_atomic_errors() {
    for opcode in [0xF5F, 0xF80, 0xFCA, 0xFCC, 0xFDA, 0xFDC] {
        let mut cpu = program(&[opcode]);
        cpu.state.sp = 0xFF;
        cpu.state.npc = 0x1A00;
        cpu.state.interrupt_delay = true;
        cpu.ram = [9; RAM_NIBBLES];
        cpu.lcd = [7; LCD_NIBBLES];
        set_flags(&mut cpu.state, 15);
        let before = cpu.clone();
        assert_eq!(
            cpu.step(),
            Err(CpuError::UnsupportedOpcode { pc: 0x100, opcode })
        );
        assert_eq!(cpu, before);
    }
}
