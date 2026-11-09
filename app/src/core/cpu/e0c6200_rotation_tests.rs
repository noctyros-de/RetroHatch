// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

#[test]
fn all_original_rotation_encodings_decode_including_rlc_aliases() {
    for opcode in 0xAF0..=0xAFF {
        assert_eq!(
            Instruction::decode(opcode),
            Some(Instruction::RotateLeft(Operand::from_bits(opcode)))
        );
    }
    for opcode in 0xE8C..=0xE8F {
        assert_eq!(
            Instruction::decode(opcode),
            Some(Instruction::RotateRight(Operand::from_bits(opcode)))
        );
    }
}

#[test]
fn rotations_match_python_for_all_encodings_values_flags_and_layouts() {
    let layouts = [
        (0x20, 0x21),
        (0x20, 0x20),
        (0x2FF, 0),
        (0xE00, 0xE4F),
        (0xE80, 0xECF),
        (0xE4F, 0xE4F),
    ];
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let mut comparisons = [0; 6];
    for row in include_str!("../../../tests/data/e0c6200_rotation_reference.txt").lines() {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('|').collect();
        let opcode = hex(fields[0]);
        let layout = hex(fields[1]) as usize;
        let initial_flags = hex(fields[2]);
        let cycles = hex(fields[3]) as u8;
        let next_pc = hex(fields[4]);
        let next_npc = hex(fields[5]);
        let delay = hex(fields[6]) != 0;
        let (ix, iy) = layouts[layout];
        let operand = opcode & 3;
        assert_eq!(fields[7].len(), 32);
        for value in 0..16 {
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
            cpu.write_operand(Operand::from_bits(operand), value)
                .unwrap();
            let offset = usize::from(value) * 2;
            let result = hex(&fields[7][offset..offset + 1]) as u8;
            let expected_flags = hex(&fields[7][offset + 1..offset + 2]);
            let mut expected_state = cpu.state.clone();
            let mut expected_ram = cpu.ram;
            let mut expected_lcd = cpu.lcd;
            match operand {
                0 => expected_state.a = result,
                1 => expected_state.b = result,
                2 | 3 => {
                    let address = if operand == 2 { ix } else { iy };
                    match address {
                        0 | 0x20 | 0x21 | 0x2FF => expected_ram[address as usize] = result,
                        0xE00 => expected_lcd[0] = result,
                        0xE4F => expected_lcd[0x4F] = result,
                        0xE80 => expected_lcd[0x50] = result,
                        0xECF => expected_lcd[0x9F] = result,
                        _ => panic!("invalid rotation layout"),
                    }
                }
                _ => unreachable!(),
            }
            set_flags(&mut expected_state, expected_flags);
            expected_state.pc = next_pc;
            expected_state.npc = next_npc;
            expected_state.interrupt_delay = delay;
            let context =
                format!("{opcode:03X}, layout={layout}, value={value:X}, flags={initial_flags:X}");
            assert_eq!(cpu.step(), Ok(Step { pc, opcode, cycles }), "{context}");
            assert_eq!(cpu.state, expected_state, "{context}");
            assert_eq!(cpu.ram, expected_ram, "{context}");
            assert_eq!(cpu.lcd, expected_lcd, "{context}");
            comparisons[layout] += 1;
        }
    }
    assert_eq!(comparisons, [5120; 6]);
}

#[test]
fn carry_rotation_edges_preserve_zero_and_five_steps_restore_carry_and_value() {
    for (opcode, value, initial_flags, expected, expected_flags, cycles) in [
        (0xAF0, 0, 0xF, 1, 0xE, 7), // Carry enters bit 0.
        (0xAF0, 8, 0xC, 0, 0xD, 7), // Result zero must not set Z.
        (0xE8C, 1, 0xC, 0, 0xD, 5), // Old bit 0 becomes carry; Z remains clear.
        (0xE8C, 0, 0xF, 8, 0xE, 5), // Carry enters bit 3; Z remains set.
    ] {
        let mut cpu = program(&[opcode]);
        cpu.state.a = value;
        set_flags(&mut cpu.state, initial_flags);
        assert_eq!(cpu.step().unwrap().cycles, cycles);
        assert_eq!((cpu.state.a, flags(&cpu.state)), (expected, expected_flags));
    }
    for opcode in [0xAF0, 0xE8C] {
        for value in 0..16 {
            for carry in [false, true] {
                let mut cpu = program(&[opcode; 5]);
                cpu.state.a = value;
                let initial_flags = 0xE | u16::from(carry);
                set_flags(&mut cpu.state, initial_flags);
                for _ in 0..5 {
                    cpu.step().unwrap();
                }
                assert_eq!(
                    (cpu.state.a, flags(&cpu.state)),
                    (value, initial_flags as u8)
                );
                assert_eq!((cpu.state.pc, cpu.state.npc), (0x105, 0x105));
            }
        }
    }
}

#[test]
fn invalid_rotation_memory_is_an_atomic_error() {
    for opcode in (0xAF0..=0xAFF).chain(0xE8C..=0xE8F) {
        let operand = opcode & 3;
        if operand < 2 {
            continue;
        }
        for address in [0x300, 0xE50, 0xE7F, 0xED0, 0xF00] {
            let mut cpu = program(&[opcode]);
            cpu.state.ix = if operand == 2 { address } else { 0x20 };
            cpu.state.iy = if operand == 3 { address } else { 0x21 };
            cpu.state.npc = 0x1A00;
            cpu.state.interrupt_delay = true;
            set_flags(&mut cpu.state, 15);
            cpu.write_memory(0x20, 8).unwrap();
            cpu.write_memory(0x21, 1).unwrap();
            let before = cpu.clone();
            assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(address)));
            assert_eq!(cpu, before);
        }
    }
}
