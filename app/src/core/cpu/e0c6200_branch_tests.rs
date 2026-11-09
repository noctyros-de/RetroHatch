use super::*;

#[test]
fn every_jump_variant_matches_the_original_for_all_targets_flags_and_page_contexts() {
    assert_eq!(
        transfers::check_reference(include_str!(
            "../../../tests/data/e0c6200_branches_reference.txt"
        )),
        98304
    );
}

#[test]
fn conditional_jump_encodings_select_the_exact_flag_and_condition() {
    for target in 0..256 {
        for (base, condition) in [
            (0x200, Condition::CarrySet),
            (0x300, Condition::CarryClear),
            (0x600, Condition::ZeroSet),
            (0x700, Condition::ZeroClear),
        ] {
            assert_eq!(
                Instruction::decode(base | target),
                Some(Instruction::JumpIf(condition, target as u8))
            );
        }
    }
    assert_eq!(Instruction::decode(0xFE8), Some(Instruction::JumpBa));
}

#[test]
fn taken_jumps_preserve_full_npc_while_untaken_jumps_discard_pending_page() {
    for (initial_flags, pc, npc) in [(1, 0x1AAB, 0x1A55), (0, 0x101, 0x101)] {
        let mut cpu = program(&[0x2AB]);
        set_flags(&mut cpu.state, initial_flags);
        cpu.state.npc = 0x1A55;
        cpu.state.interrupt_delay = true;
        assert_eq!(cpu.step().unwrap().cycles, 5);
        assert_eq!((cpu.state.pc, cpu.state.npc), (pc, npc));
        assert_eq!(flags(&cpu.state), initial_flags as u8);
        assert!(!cpu.state.interrupt_delay);
    }
}

#[test]
fn a_jump_does_not_fetch_the_destination_until_the_next_step() {
    for opcode in [0x0AB, 0x2AB, 0xFE8] {
        let mut cpu = program(&[opcode]);
        cpu.state.npc = 0x1F00;
        cpu.state.carry = true;
        cpu.step().unwrap();
        assert_eq!(cpu.state.pc & 0x1F00, 0x1F00);
        let before = cpu.clone();
        assert_eq!(
            cpu.step(),
            Err(CpuError::MissingInstruction { pc: cpu.state.pc })
        );
        assert_eq!(cpu, before);
    }
}
