use super::*;

fn program(words: &[u16]) -> Cpu {
    let mut code = vec![0x100; 0x100];
    code.extend_from_slice(words);
    Cpu::new(&code).unwrap()
}

fn hex(value: &str) -> u16 {
    u16::from_str_radix(value, 16).unwrap()
}

fn flags(state: &CpuState) -> u8 {
    u8::from(state.carry)
        | (u8::from(state.zero) << 1)
        | (u8::from(state.decimal) << 2)
        | (u8::from(state.interrupt_enable) << 3)
}

fn sparse<const N: usize>(value: &str) -> [u8; N] {
    let mut memory = [0; N];
    if !value.is_empty() {
        for pair in value.split(',') {
            let (address, nibble) = pair.split_once(':').unwrap();
            memory[hex(address) as usize] = hex(nibble) as u8;
        }
    }
    memory
}

#[test]
fn reset_registers_and_memory_match_reference() {
    let cpu = program(&[]);
    assert_eq!(cpu.state(), &CpuState::default());
    assert_eq!(cpu.state.pc, 0x100);
    assert_eq!(cpu.state.npc, 0x100);
    assert!(cpu.ram().iter().all(|&v| v == 0));
    assert!(cpu.lcd_memory().iter().all(|&v| v == 0));
}

#[test]
fn synthetic_programs_match_python_after_every_instruction() {
    let expected = include_str!("../../../tests/data/e0c6200_reference.txt");
    let mut comparisons = 0;
    for line in include_str!("../../../tests/data/e0c6200_programs.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        let name = fields[0];
        let mut code = vec![0x100; PROGRAM_WORDS];
        for pair in fields[10].split(',') {
            let (address, word) = pair.split_once(':').unwrap();
            code[hex(address) as usize] = hex(word);
        }
        let mut cpu = Cpu::new(&code).unwrap();
        cpu.state.pc = hex(fields[1]);
        cpu.state.npc = hex(fields[2]);
        cpu.state.a = hex(fields[3]) as u8;
        cpu.state.b = hex(fields[4]) as u8;
        cpu.state.ix = hex(fields[5]);
        cpu.state.iy = hex(fields[6]);
        cpu.state.sp = hex(fields[7]) as u8;
        let initial_flags = hex(fields[8]);
        cpu.state.carry = initial_flags & 1 != 0;
        cpu.state.zero = initial_flags & 2 != 0;
        cpu.state.decimal = initial_flags & 4 != 0;
        cpu.state.interrupt_enable = initial_flags & 8 != 0;
        let rows: Vec<_> = expected
            .lines()
            .filter(|row| row.starts_with(&format!("{name}|")))
            .collect();
        assert_eq!(rows.len(), fields[9].parse::<usize>().unwrap());
        for (index, row) in rows.iter().enumerate() {
            let data: Vec<_> = row.split('|').collect();
            let before_pc = cpu.state.pc;
            let step = cpu.step().unwrap();
            assert_eq!(step.pc, before_pc);
            assert_eq!(step.opcode, code[before_pc as usize]);
            let actual = [
                index as u16,
                u16::from(step.cycles),
                u16::from(cpu.state.a),
                u16::from(cpu.state.b),
                cpu.state.ix,
                cpu.state.iy,
                u16::from(cpu.state.sp),
                cpu.state.pc,
                cpu.state.npc,
                u16::from(flags(&cpu.state)),
                u16::from(cpu.state.interrupt_delay),
                u16::from(cpu.state.halted),
            ];
            let wanted: Vec<_> = data[1..13].iter().map(|v| hex(v)).collect();
            assert_eq!(actual.as_slice(), wanted, "{name}, instruction {index}");
            assert_eq!(
                cpu.ram,
                sparse::<RAM_NIBBLES>(data[13]),
                "{name}, RAM step {index}"
            );
            assert_eq!(
                cpu.lcd,
                sparse::<LCD_NIBBLES>(data[14]),
                "{name}, LCD step {index}"
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 174);
}

#[test]
fn add_matches_python_for_all_nibbles_decimal_modes_and_operands() {
    let mut comparisons = 0;
    for line in include_str!("../../../tests/data/e0c6200_add_reference.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let values: Vec<_> = line.split('|').map(hex).collect();
        for operand in 0..4 {
            let mut cpu = program(&[0xC00 | (operand << 4) | values[2]]);
            cpu.state.ix = 0x20;
            cpu.state.iy = 0x21;
            cpu.state.decimal = values[0] != 0;
            cpu.state.interrupt_enable = true;
            // ADD must not include the incoming carry.
            cpu.state.carry = true;
            let target = Operand::from_bits(operand);
            cpu.write_operand(target, values[1] as u8).unwrap();
            let result = cpu.step().unwrap();
            assert_eq!(result.cycles, 7);
            assert_eq!(cpu.read_operand(target).unwrap(), values[3] as u8);
            assert_eq!(cpu.state.carry, values[4] != 0);
            assert_eq!(cpu.state.zero, values[5] != 0);
            assert_eq!(cpu.state.decimal, values[0] != 0);
            assert!(cpu.state.interrupt_enable);
            assert_eq!((cpu.state.pc, cpu.state.npc), (0x101, 0x101));
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 2048);
}

#[test]
fn all_register_copy_combinations_read_source_before_write() {
    for target in 0..4 {
        for source in 0..4 {
            let mut cpu = program(&[0xEC0 | (target << 2) | source]);
            cpu.state.ix = 0x10;
            cpu.state.iy = 0x11;
            for operand in 0..4 {
                cpu.write_operand(Operand::from_bits(operand), operand as u8 + 1)
                    .unwrap();
            }
            let step = cpu.step().unwrap();
            assert_eq!(step.cycles, 5);
            assert_eq!(
                cpu.read_operand(Operand::from_bits(target)).unwrap(),
                source as u8 + 1
            );
        }
    }
    let mut cpu = program(&[0xECB]); // M(X) <- M(Y), same physical memory location.
    cpu.state.ix = 5;
    cpu.state.iy = 5;
    cpu.write_memory(5, 9).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(5), Ok(9));
}

#[test]
fn every_supported_immediate_and_address_field_decodes_without_truncation() {
    for value in 0..=255 {
        assert_eq!(
            Instruction::decode(0x800 | value),
            Some(Instruction::LoadY(value as u8))
        );
        assert_eq!(
            Instruction::decode(0xB00 | value),
            Some(Instruction::LoadX(value as u8))
        );
        assert_eq!(
            Instruction::decode(value),
            Some(Instruction::Jump(value as u8))
        );
    }
    for page in 0..32 {
        assert_eq!(
            Instruction::decode(0xE40 | page),
            Some(Instruction::SetPage(page as u8))
        );
    }
    for operand in 0..4 {
        for value in 0..16 {
            let mut cpu = program(&[0xE00 | (operand << 4) | value]);
            cpu.state.iy = 1;
            cpu.step().unwrap();
            assert_eq!(
                cpu.read_operand(Operand::from_bits(operand)),
                Ok(value as u8)
            );
        }
    }
}

#[test]
fn memory_boundaries_are_nibble_wide_and_lcd_windows_are_contiguous() {
    let mut cpu = program(&[]);
    for (address, lcd_index) in [(0xE00, 0), (0xE4F, 0x4F), (0xE80, 0x50), (0xECF, 0x9F)] {
        cpu.write_memory(address, 0xAB).unwrap();
        assert_eq!(cpu.read_memory(address), Ok(0xB));
        assert_eq!(cpu.lcd[lcd_index], 0xB);
    }
    for address in [0, 0x2FF] {
        cpu.write_memory(address, 0xFF).unwrap();
        assert_eq!(cpu.read_memory(address), Ok(15));
    }
    for address in [0x300, 0xDFF, 0xE50, 0xE7F, 0xED0, 0xF00, 0xFFF, 0x1000] {
        let before = cpu.clone();
        assert_eq!(
            cpu.read_memory(address),
            Err(CpuError::UnsupportedAddress(address))
        );
        assert_eq!(
            cpu.write_memory(address, 7),
            Err(CpuError::UnsupportedAddress(address))
        );
        assert_eq!(cpu, before);
    }
}

#[test]
fn unsupported_opcodes_are_explicit_and_leave_everything_unchanged() {
    let mut supported = 0;
    let mut cpu = program(&[0x100]);
    cpu.state.interrupt_delay = true;
    for opcode in 0..=u16::MAX {
        let allowed = matches!(opcode, 0x000..=0x0FF | 0x800..=0x8FF | 0xA00..=0xAEF
            | 0xB00..=0xBFF | 0xC00..=0xDFF | 0xE00..=0xE7F | 0xEC0..=0xECF
            | 0xF00..=0xF1F | 0xFFB | 0xFFF);
        assert_eq!(
            Instruction::decode(opcode).is_some(),
            allowed,
            "{opcode:04X}"
        );
        if allowed {
            supported += 1;
        } else if opcode <= 0xFFF {
            cpu.program[0x100] = opcode;
            let before = cpu.clone();
            assert_eq!(
                cpu.step(),
                Err(CpuError::UnsupportedOpcode { pc: 0x100, opcode })
            );
            assert_eq!(cpu, before);
        }
    }
    assert_eq!(supported, 1698);
}

fn set_flags(state: &mut CpuState, value: u16) {
    state.carry = value & 1 != 0;
    state.zero = value & 2 != 0;
    state.decimal = value & 4 != 0;
    state.interrupt_enable = value & 8 != 0;
}

#[test]
fn immediate_alu_matches_python_for_every_flag_operand_and_nibble() {
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let mut comparisons = 0;
    for line in include_str!("../../../tests/data/e0c6200_immediate_alu_reference.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        let base = hex(fields[0]);
        let target = hex(fields[1]);
        let initial_flags = hex(fields[2]);
        let initial_value = hex(fields[3]) as u8;
        let cycles = hex(fields[4]) as u8;
        let next_pc = hex(fields[5]);
        let next_npc = hex(fields[6]);
        let delay = hex(fields[7]) != 0;
        assert_eq!(fields[8].len(), 32);
        for immediate in 0..16 {
            let pc = [0x100, 0xFFF, 0x1FFF, 0x10FF][(initial_flags & 3) as usize];
            let opcode = base | (target << 4) | immediate;
            cpu.program[pc as usize] = opcode;
            cpu.state = CpuState {
                a: 3,
                b: 5,
                ix: 0x20,
                iy: 0x21,
                sp: 0xA5,
                pc,
                npc: 0x1A00,
                // Successful ALU steps clear the interrupt-delay flag.
                interrupt_delay: true,
                ..CpuState::default()
            };
            set_flags(&mut cpu.state, initial_flags);
            cpu.ram = std::array::from_fn(|i| ((i * 3 + 1) & 15) as u8);
            cpu.lcd = std::array::from_fn(|i| ((i + 5) & 15) as u8);
            cpu.write_operand(Operand::from_bits(target), initial_value)
                .unwrap();

            let offset = immediate as usize * 2;
            let result = hex(&fields[8][offset..offset + 1]) as u8;
            let expected_flags = hex(&fields[8][offset + 1..offset + 2]);
            let mut expected_state = cpu.state.clone();
            let mut expected_ram = cpu.ram;
            let expected_lcd = cpu.lcd;
            match target {
                0 => expected_state.a = result,
                1 => expected_state.b = result,
                2 => expected_ram[0x20] = result,
                3 => expected_ram[0x21] = result,
                _ => panic!("invalid target in reference data"),
            }
            set_flags(&mut expected_state, expected_flags);
            expected_state.pc = next_pc;
            expected_state.npc = next_npc;
            expected_state.interrupt_delay = delay;
            let context =
                format!("opcode {opcode:03X}, value {initial_value:X}, flags {initial_flags:X}");
            assert_eq!(cpu.step(), Ok(Step { pc, opcode, cycles }), "{context}");
            assert_eq!(cpu.state, expected_state, "{context}");
            assert_eq!(cpu.ram, expected_ram, "{context}");
            assert_eq!(cpu.lcd, expected_lcd, "{context}");
            comparisons += 1;
        }
    }
    // Eight instructions, four targets, 16 flag combinations, 16 values, 16 immediates.
    assert_eq!(comparisons, 131072);
}

#[test]
fn immediate_alu_decode_matches_all_operand_and_immediate_fields() {
    for target in 0..4 {
        for immediate in 0..16 {
            let operand = Operand::from_bits(target);
            let value = immediate as u8;
            for (base, expected) in [
                (0xC40, Instruction::AdcImmediate(operand, value)),
                (0xC80, Instruction::AndImmediate(operand, value)),
                (0xCC0, Instruction::OrImmediate(operand, value)),
                (0xD00, Instruction::XorImmediate(operand, value)),
                (0xD40, Instruction::SbcImmediate(operand, value)),
                (0xD80, Instruction::FanImmediate(operand, value)),
                (0xDC0, Instruction::CompareImmediate(operand, value)),
            ] {
                assert_eq!(
                    Instruction::decode(base | (target << 4) | immediate),
                    Some(expected)
                );
            }
        }
    }
}

#[test]
fn decoder_coverage_matches_original_python_handler_inventory() {
    let mut next_address = 0;
    let mut supported = 0;
    for row in include_str!("../../../tests/data/e0c6200_decode_reference.txt").lines() {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('|').collect();
        let start = hex(fields[0]);
        let end = hex(fields[1]);
        assert_eq!(start, next_address);
        let expected_support = fields[3] == "1";
        for opcode in start..=end {
            assert_eq!(
                Instruction::decode(opcode).is_some(),
                expected_support,
                "Python handler {} at {opcode:03X}",
                fields[2]
            );
            supported += usize::from(expected_support);
        }
        next_address = end + 1;
    }
    assert_eq!(next_address, 4096);
    assert_eq!(supported, 1698);
}

#[test]
fn carry_borrow_and_non_bcd_boundaries_match_reference_rules() {
    // Opcode, operand, initial flags, result, final flags. I remains set.
    for (opcode, initial, initial_flags, result, expected_flags) in [
        (0xC40, 15, 0x9, 0, 0xB), // 15+0+C = 16
        (0xC4F, 15, 0xD, 5, 0xD), // 15+15+C+6 = 37; invalid BCD values
        (0xD4F, 0, 0x9, 0, 0xB),  // 0-15-C = -16, nibble 0, borrow remains
        (0xD4F, 0, 0xD, 10, 0xD), // -16+10 = -6, nibble A
        (0xD4F, 5, 0xC, 0, 0xF),  // 5-15+10 = 0 with borrow
        (0xD40, 0, 0x8, 0, 0xA),  // No carry-in: 0-0 = 0 without borrow
    ] {
        let mut cpu = program(&[opcode]);
        cpu.state.a = initial;
        set_flags(&mut cpu.state, initial_flags);
        assert_eq!(cpu.step().unwrap().cycles, 7);
        assert_eq!((cpu.state.a, flags(&cpu.state)), (result, expected_flags));
    }
}

#[test]
fn fan_and_compare_do_not_write_operands_and_logic_preserves_carry() {
    for (opcode, initial, initial_flags, result, expected_flags) in [
        (0xD80, 9, 0xD, 9, 0xF), // FAN 9,0: set Z, preserve carry
        (0xD8F, 9, 0xF, 9, 0xD), // FAN 9,F: clear Z
        (0xDC9, 9, 0xD, 9, 0xE), // CP equal: C=0, Z=1
        (0xDCF, 9, 0xE, 9, 0xD), // CP less than: C=1, Z=0
        (0xDC0, 9, 0xF, 9, 0xC), // CP greater than: C=0, Z=0
        (0xC80, 9, 0xD, 0, 0xF), // AND writes; only Z changes
        (0xCC1, 0, 0xF, 1, 0xD), // OR
        (0xD09, 9, 0xD, 0, 0xF), // XOR
    ] {
        let mut cpu = program(&[opcode]);
        cpu.state.a = initial;
        set_flags(&mut cpu.state, initial_flags);
        cpu.step().unwrap();
        assert_eq!((cpu.state.a, flags(&cpu.state)), (result, expected_flags));
    }
}

#[test]
fn immediate_alu_rejects_missing_memory_without_partial_flag_or_pc_changes() {
    for base in [0xC00, 0xC40, 0xC80, 0xCC0, 0xD00, 0xD40, 0xD80, 0xDC0] {
        for target in [2, 3] {
            for address in [0x300, 0xE50, 0xF00] {
                for immediate in [0, 15] {
                    let mut cpu = program(&[base | (target << 4) | immediate]);
                    if target == 2 {
                        cpu.state.ix = address;
                    } else {
                        cpu.state.iy = address;
                    }
                    set_flags(&mut cpu.state, 15);
                    cpu.state.interrupt_delay = true;
                    let before = cpu.clone();
                    assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(address)));
                    assert_eq!(cpu, before);
                }
            }
        }
    }
}

#[test]
fn unsupported_memory_in_instructions_does_not_partially_commit() {
    for opcode in [0xE21, 0xC21, 0xEC2, 0xEC8, 0xE61] {
        let mut cpu = program(&[opcode]);
        cpu.state.ix = 0xF00;
        cpu.state.interrupt_delay = true;
        cpu.state.carry = true;
        let before = cpu.clone();
        assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(0xF00)));
        assert_eq!(cpu, before);
    }
}

#[test]
fn invalid_programs_and_missing_fetch_are_errors() {
    assert_eq!(
        Cpu::new(&[0x1000]),
        Err(CpuError::InvalidProgramWord {
            address: 0,
            value: 0x1000
        })
    );
    assert_eq!(
        Cpu::new(&vec![0; PROGRAM_WORDS + 1]),
        Err(CpuError::ProgramTooLarge(PROGRAM_WORDS + 1))
    );
    let mut cpu = Cpu::new(&[]).unwrap();
    let before = cpu.clone();
    assert_eq!(cpu.step(), Err(CpuError::MissingInstruction { pc: 0x100 }));
    assert_eq!(cpu, before);
}

#[path = "e0c6200_two_operand_tests.rs"]
mod two_operand;

#[path = "e0c6200_index_tests.rs"]
mod index;
