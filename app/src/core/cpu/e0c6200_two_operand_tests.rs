use super::*;

const OPERATIONS: [(u16, AluOperation); 9] = [
    (0xA80, AluOperation::Add),
    (0xA90, AluOperation::Adc),
    (0xAA0, AluOperation::Sub),
    (0xAB0, AluOperation::Sbc),
    (0xAC0, AluOperation::And),
    (0xAD0, AluOperation::Or),
    (0xAE0, AluOperation::Xor),
    (0xF00, AluOperation::Compare),
    (0xF10, AluOperation::Fan),
];

#[test]
fn every_two_operand_encoding_decodes_target_and_source() {
    for (base, operation) in OPERATIONS {
        for target in 0..4 {
            for source in 0..4 {
                assert_eq!(
                    Instruction::decode(base | (target << 2) | source),
                    Some(Instruction::Alu(
                        operation,
                        Operand::from_bits(target),
                        Operand::from_bits(source)
                    ))
                );
            }
        }
    }
}

#[test]
fn two_operand_alu_matches_original_for_all_values_flags_pairs_and_aliases() {
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let mut comparisons = [0; 4];
    for line in include_str!("../../../tests/data/e0c6200_two_operand_alu_reference.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('|').collect();
        let opcode = hex(fields[0]);
        let target = (opcode >> 2) & 3;
        let source = opcode & 3;
        let layout = hex(fields[1]) as usize;
        let initial_flags = hex(fields[2]);
        let left = hex(fields[3]) as u8;
        let cycles = hex(fields[5]) as u8;
        let next_pc = hex(fields[6]);
        let next_npc = hex(fields[7]);
        let delay = hex(fields[8]) != 0;
        let (ix, iy) = [(0x20, 0x21), (0x20, 0x20), (0xE4F, 0xECF), (0xE80, 0xE80)][layout];
        assert_eq!(fields[9].len(), fields[4].len() * 2);
        for (index, value) in fields[4].as_bytes().iter().enumerate() {
            let right = hex(std::str::from_utf8(std::slice::from_ref(value)).unwrap()) as u8;
            let pc = [0x100, 0xFFF, 0x1FFF, 0x10FF][(initial_flags & 3) as usize];
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
            cpu.ram = std::array::from_fn(|i| ((i * 3 + 1) & 15) as u8);
            cpu.lcd = std::array::from_fn(|i| ((i + 5) & 15) as u8);
            cpu.write_operand(Operand::from_bits(target), left).unwrap();
            cpu.write_operand(Operand::from_bits(source), right)
                .unwrap();
            // Initial values must be consistent when both operands share storage.
            assert_eq!(cpu.read_operand(Operand::from_bits(target)), Ok(left));
            assert_eq!(cpu.read_operand(Operand::from_bits(source)), Ok(right));

            let offset = index * 2;
            let result = hex(&fields[9][offset..offset + 1]) as u8;
            let expected_flags = hex(&fields[9][offset + 1..offset + 2]);
            let mut expected_state = cpu.state.clone();
            let mut expected_ram = cpu.ram;
            let mut expected_lcd = cpu.lcd;
            match target {
                0 => expected_state.a = result,
                1 => expected_state.b = result,
                2 | 3 => {
                    let address = if target == 2 { ix } else { iy };
                    match address {
                        0x20 | 0x21 => expected_ram[address as usize] = result,
                        0xE4F => expected_lcd[0x4F] = result,
                        0xECF => expected_lcd[0x9F] = result,
                        0xE80 => expected_lcd[0x50] = result,
                        _ => panic!("invalid address in test layout"),
                    }
                }
                _ => unreachable!(),
            }
            set_flags(&mut expected_state, expected_flags);
            expected_state.pc = next_pc;
            expected_state.npc = next_npc;
            expected_state.interrupt_delay = delay;
            let context = format!(
                "{opcode:03X}, layout {layout}, flags {initial_flags:X}, left {left:X}, right {right:X}"
            );
            assert_eq!(cpu.step(), Ok(Step { pc, opcode, cycles }), "{context}");
            assert_eq!(cpu.state, expected_state, "{context}");
            assert_eq!(cpu.ram, expected_ram, "{context}");
            assert_eq!(cpu.lcd, expected_lcd, "{context}");
            comparisons[layout] += 1;
        }
    }
    assert_eq!(comparisons, [451584, 4608, 24192, 1152]);
}

#[test]
fn two_operand_carry_borrow_and_alias_boundaries() {
    // A op B, initial flags, result A, final flags. ADD/SUB ignore carry-in.
    for (opcode, a, b, initial_flags, result, expected_flags) in [
        (0xA81, 15, 1, 0x9, 0, 0xB),
        (0xA91, 15, 0, 0x9, 0, 0xB),
        (0xAA1, 0, 0, 0x9, 0, 0xA),
        (0xAB1, 0, 0, 0x9, 15, 0x9),
        (0xAA1, 0, 15, 0xD, 11, 0xD),
        (0xAB1, 0, 15, 0xD, 10, 0xD),
        (0xA91, 15, 15, 0xD, 5, 0xD),
        (0xF01, 9, 9, 0xD, 9, 0xE), // CP: no write; clear C.
        (0xF11, 9, 6, 0xD, 9, 0xF), // FAN: no write; preserve C.
    ] {
        let mut cpu = program(&[opcode]);
        cpu.state.a = a;
        cpu.state.b = b;
        set_flags(&mut cpu.state, initial_flags);
        assert_eq!(cpu.step().unwrap().cycles, 7);
        assert_eq!(
            (cpu.state.a, cpu.state.b, flags(&cpu.state)),
            (result, b, expected_flags)
        );
    }

    let mut cpu = program(&[0xA8B, 0xABA, 0xF1B]); // ADD MX,MY; SBC MX,MX; FAN MX,MY
    cpu.state.ix = 0x20;
    cpu.state.iy = 0x20;
    cpu.write_memory(0x20, 8).unwrap();
    cpu.step().unwrap();
    assert_eq!(
        (cpu.read_memory(0x20), cpu.state.carry, cpu.state.zero),
        (Ok(0), true, true)
    );
    cpu.step().unwrap();
    assert_eq!(
        (cpu.read_memory(0x20), cpu.state.carry, cpu.state.zero),
        (Ok(15), true, false)
    );
    cpu.step().unwrap();
    assert_eq!(
        (cpu.read_memory(0x20), cpu.state.carry, cpu.state.zero),
        (Ok(15), true, false)
    );
}

#[test]
fn unsupported_two_operand_memory_never_partially_commits() {
    for (base, _) in OPERATIONS {
        for target in 0..4 {
            for source in 0..4 {
                for invalid_operand in [2, 3] {
                    if target != invalid_operand && source != invalid_operand {
                        continue;
                    }
                    for address in [0x300, 0xE50, 0xF00] {
                        let opcode = base | (target << 2) | source;
                        let mut cpu = program(&[opcode]);
                        cpu.state.a = 15;
                        cpu.state.b = 9;
                        cpu.state.ix = if invalid_operand == 2 { address } else { 0x20 };
                        cpu.state.iy = if invalid_operand == 3 { address } else { 0x21 };
                        cpu.state.npc = 0x1A00;
                        cpu.state.interrupt_delay = true;
                        set_flags(&mut cpu.state, 15);
                        let before = cpu.clone();
                        assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(address)));
                        assert_eq!(
                            cpu, before,
                            "{opcode:03X}, invalid operand {invalid_operand}"
                        );
                    }
                }
            }
        }
    }
}
