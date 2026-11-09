// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;

#[test]
fn halt_nops_and_every_pset_field_match_reference_cycles_flags_and_pc_boundaries() {
    assert_eq!(
        transfers::check_reference(include_str!(
            "../../../tests/data/e0c6200_control_reference.txt"
        )),
        4864
    );
}

#[test]
fn halted_steps_do_not_fetch_execute_clear_delay_or_change_memory() {
    for words in [vec![0xFF8], vec![0xFF8, 0xE0F]] {
        let mut cpu = program(&words);
        cpu.state.a = 3;
        cpu.state.npc = 0x1A55;
        cpu.state.interrupt_delay = true;
        set_flags(&mut cpu.state, 15);
        cpu.ram = [7; RAM_NIBBLES];
        cpu.lcd = [9; LCD_NIBBLES];
        assert_eq!(
            cpu.step(),
            Ok(Step {
                pc: 0x100,
                opcode: 0xFF8,
                cycles: 5
            })
        );
        assert!(cpu.state.halted);
        assert_eq!((cpu.state.pc, cpu.state.npc), (0x101, 0x101));
        assert_eq!(cpu.state.a, 3);
        assert_eq!(flags(&cpu.state), 15);
        assert!(!cpu.state.interrupt_delay);
        let before = cpu.clone();
        for _ in 0..3 {
            assert_eq!(cpu.step(), Err(CpuError::Halted { pc: 0x101 }));
            assert_eq!(cpu, before);
        }
    }
}

#[test]
fn reserved_control_encodings_remain_atomic_errors_with_enabled_interrupts() {
    for opcode in [0xFE9, 0xFEF, 0xFF9, 0xFFA, 0xFFC, 0xFFD, 0xFFE] {
        let mut cpu = program(&[opcode]);
        cpu.state.npc = 0x1A55;
        cpu.state.sp = 0xFF;
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

#[test]
fn complete_decoder_distinguishes_halt_from_both_real_nop_variants() {
    assert_eq!(Instruction::decode(0xFF8), Some(Instruction::Halt));
    assert_eq!(Instruction::decode(0xFFB), Some(Instruction::Nop5));
    assert_eq!(Instruction::decode(0xFFF), Some(Instruction::Nop7));
    assert!(
        CpuError::Halted { pc: 0x101 }
            .to_string()
            .contains("halted")
    );
}
