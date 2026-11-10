// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

const OPERATIONS: [(u16, fn(Operand) -> Instruction); 4] = [
    (0xF28, Instruction::AdcPostX),
    (0xF2C, Instruction::AdcPostY),
    (0xF38, Instruction::SbcPostX),
    (0xF3C, Instruction::SbcPostY),
];

#[test]
fn all_post_increment_opcodes_decode_the_original_source() {
    for (base, instruction) in OPERATIONS {
        for operand in 0..4 {
            assert_eq!(
                Instruction::decode(base | operand),
                Some(instruction(Operand::from_bits(operand)))
            );
        }
    }
}

#[test]
fn post_increment_matches_python_values_flags_aliases_and_boundaries() {
    let layouts = [
        (0x20, 0x21),
        (0x20, 0x20),
        (0x2FF, 0x1FF),
        (0xE4F, 0xECF),
        (0xE4F, 0xE4F),
        (0x2FF, 0xE4F),
        (0xECF, 0x2FF),
    ];
    let mut cpu = Cpu::new(&vec![0x100; PROGRAM_WORDS]).unwrap();
    let mut comparisons = [0; 7];
    for row in include_str!("../../../tests/data/e0c6200_post_increment_reference.txt").lines() {
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('|').collect();
        let opcode = hex(fields[0]);
        let layout = hex(fields[1]) as usize;
        let initial_flags = hex(fields[2]);
        let left = hex(fields[3]) as u8;
        let cycles = hex(fields[5]) as u8;
        let next_pc = hex(fields[6]);
        let next_npc = hex(fields[7]);
        let next_ix = hex(fields[8]);
        let next_iy = hex(fields[9]);
        let delay = hex(fields[10]) != 0;
        let (ix, iy) = layouts[layout];
        let is_x = opcode & 4 == 0;
        let address = if is_x { ix } else { iy };
        assert_eq!(fields[11].len(), fields[4].len() * 2);
        for (index, _) in fields[4].bytes().enumerate() {
            let right = hex(&fields[4][index..index + 1]) as u8;
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
            cpu.write_memory(address, left).unwrap();
            let operand = Operand::from_bits(opcode);
            cpu.write_operand(operand, right).unwrap();
            assert_eq!(cpu.read_memory(address), Ok(left));
            assert_eq!(cpu.read_operand(operand), Ok(right));
            let offset = index * 2;
            let result = hex(&fields[11][offset..offset + 1]) as u8;
            let expected_flags = hex(&fields[11][offset + 1..offset + 2]);
            let mut expected_state = cpu.state.clone();
            let mut expected_ram = cpu.ram;
            let mut expected_lcd = cpu.lcd;
            match address {
                0x20 | 0x21 | 0x2FF | 0x1FF => expected_ram[address as usize] = result,
                0xE4F => expected_lcd[0x4F] = result,
                0xECF => expected_lcd[0x9F] = result,
                _ => panic!("invalid reference layout"),
            }
            expected_state.ix = next_ix;
            expected_state.iy = next_iy;
            expected_state.pc = next_pc;
            expected_state.npc = next_npc;
            expected_state.interrupt_delay = delay;
            set_flags(&mut expected_state, expected_flags);
            let context = format!(
                "{opcode:03X}, layout={layout}, flags={initial_flags:X}, left={left:X}, right={right:X}"
            );
            assert_eq!(cpu.step(), Ok(Step { pc, opcode, cycles }), "{context}");
            assert_eq!(cpu.state, expected_state, "{context}");
            assert_eq!(cpu.ram, expected_ram, "{context}");
            assert_eq!(cpu.lcd, expected_lcd, "{context}");
            comparisons[layout] += 1;
        }
    }
    assert_eq!(
        comparisons,
        [50176, 34816, 50176, 50176, 34816, 50176, 50176]
    );
}

#[test]
fn post_increment_reads_old_operands_then_writes_old_address() {
    for (base, _) in OPERATIONS {
        let is_x = base & 4 == 0;
        let operand = if is_x { 2 } else { 3 }; // Source aliases the destination.
        let mut cpu = program(&[base | operand]);
        cpu.state.ix = if is_x { 0x2FF } else { 0x21 };
        cpu.state.iy = if is_x { 0x21 } else { 0x2FF };
        cpu.write_memory(0x2FF, 8).unwrap();
        cpu.write_memory(0x200, 3).unwrap(); // Must not be used as the source or overwritten.
        let subtract = base >= 0xF38;
        cpu.state.carry = subtract;
        cpu.step().unwrap();
        assert_eq!(if is_x { cpu.state.ix } else { cpu.state.iy }, 0x200);
        assert_eq!(if is_x { cpu.state.iy } else { cpu.state.ix }, 0x21);
        assert_eq!(cpu.read_memory(0x2FF), Ok(if subtract { 15 } else { 0 }));
        assert_eq!(cpu.read_memory(0x200), Ok(3));
        assert!(cpu.state.carry);
        assert_eq!(cpu.state.zero, !subtract);
    }
}

#[test]
fn post_increment_decimal_borrow_and_lcd_gap_follow_reference() {
    for (opcode, left, right, initial_flags, expected, expected_flags) in [
        (0xF28, 15, 15, 0xD, 5, 0xD), // 15+15+C+6 = 37 -> 5.
        (0xF2C, 9, 0, 0xD, 0, 0xF),   // 9+0+C+6 = 16 -> 0.
        (0xF38, 0, 15, 0xD, 10, 0xD), // -16+10 = -6 -> A.
        (0xF3C, 5, 15, 0xC, 0, 0xF),  // -10+10 = 0 with borrow.
    ] {
        let is_x = opcode & 4 == 0;
        let address = if is_x { 0xE4F } else { 0xECF };
        let next = address + 1;
        let access = if is_x { 0xEC2 } else { 0xEC3 }; // Read updated M(X)/M(Y).
        let mut cpu = program(&[opcode, access]);
        cpu.state.ix = if is_x { address } else { 0x20 };
        cpu.state.iy = if is_x { 0x20 } else { address };
        cpu.state.a = right;
        set_flags(&mut cpu.state, initial_flags);
        cpu.write_memory(address, left).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.read_memory(address), Ok(expected));
        assert_eq!(flags(&cpu.state), expected_flags);
        assert_eq!(if is_x { cpu.state.ix } else { cpu.state.iy }, next);
        let before = cpu.clone();
        assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(next)));
        assert_eq!(cpu, before);
    }
}

#[test]
fn post_increment_memory_errors_never_change_flags_memory_or_pointers() {
    for (base, _) in OPERATIONS {
        let is_x = base & 4 == 0;
        for operand in 0..4 {
            for invalid_target in [false, true] {
                if !invalid_target && (operand < 2 || (operand == 2) == is_x) {
                    continue;
                }
                for bad_address in [0x300, 0xE50, 0xF00] {
                    let mut cpu = program(&[base | operand]);
                    cpu.state.ix = 0x20;
                    cpu.state.iy = 0x21;
                    let invalid_x = if invalid_target { is_x } else { operand == 2 };
                    if invalid_x {
                        cpu.state.ix = bad_address;
                    } else {
                        cpu.state.iy = bad_address;
                    }
                    cpu.write_memory(0x20, 15).unwrap();
                    cpu.write_memory(0x21, 9).unwrap();
                    cpu.state.npc = 0x1A00;
                    cpu.state.interrupt_delay = true;
                    set_flags(&mut cpu.state, 15);
                    let before = cpu.clone();
                    assert_eq!(cpu.step(), Err(CpuError::UnsupportedAddress(bad_address)));
                    assert_eq!(cpu, before);
                }
            }
        }
    }
}
