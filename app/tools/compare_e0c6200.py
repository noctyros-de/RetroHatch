# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Generate reproducible CPU-only fixtures from unchanged BrickEmuPy handlers.

Uses only synthetic instructions; no firmware, imports, constructor, files in
references/, peripherals, clocks or GUI. Run with Python 3 from any directory.
The Rust unit tests consume the checked-in results without requiring Python.
"""

import argparse
import ast
import hashlib
from pathlib import Path
from workspace_paths import references_dir
REFERENCES = references_dir()

APP = Path(__file__).resolve().parents[1]
SOURCE = REFERENCES / "BrickEmuPy-main/cores/E0C6200.py"
DATA = APP / "tests/data"


def reference_class():
    source = SOURCE.read_bytes()
    tree = ast.parse(source, filename=str(SOURCE))
    cls = next(n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == "E0C6200")
    namespace = {"RAM_SIZE": 0x300, "VRAM_PART1_OFFSET": 0xE00,
                 "VRAM_PART2_OFFSET": 0xE80, "VRAM_PART_SIZE": 0x50,
                 "IORAM_OFFSET": 0xF00, "IORAM_SIZE": 0x7F}
    exec(compile(ast.Module(body=[cls], type_ignores=[]), str(SOURCE), "exec"), namespace)
    init = next(n for n in cls.body if isinstance(n, ast.FunctionDef) and n.name == "__init__")
    names = {"_execute", "_get_abmxmy_tbl", "_set_abmxmy_tbl"}
    tables = [n for n in init.body if isinstance(n, ast.Assign)
              and isinstance(n.targets[0], ast.Attribute) and n.targets[0].attr in names]
    assert len(tables) == 3
    return namespace, compile(ast.Module(body=tables, type_ignores=[]), str(SOURCE), "exec"), source


def machine(namespace, tables, values):
    cls = namespace["E0C6200"]
    cpu = cls.__new__(cls)
    pc, npc, a, b, ix, iy, sp, flags = values
    for name, value in zip(("_PC", "_NPC", "_A", "_B", "_IX", "_IY", "_SP"),
                           (pc, npc, a, b, ix, iy, sp)):
        setattr(cpu, name, value)
    cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (bool(flags & (1 << bit)) for bit in range(4))
    cpu._HALT = 0
    cpu._if_delay = False
    cpu._RAM = [0] * 0x300
    cpu._VRAM = [0] * 0xA0
    exec(tables, {**namespace, "self": cpu})
    return cpu


def flags(cpu):
    return sum(int(bool(getattr(cpu, name))) << bit
               for bit, name in enumerate(("_CF", "_ZF", "_DF", "_IF")))


def sparse(values):
    return ",".join(f"{i:X}:{v:X}" for i, v in enumerate(values) if v)


def immediate_alu_matrix(namespace, tables, source):
    """Direct original-handler results for every flag/operand/nibble combination.

    One row packs the 16 immediate values as result/flags nibble pairs.
    Unchanged registers and complete memory are independently asserted in Python;
    Rust reconstructs and checks the full expected state and memory as well.
    """
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# base|target|initial_flags|initial_value|cycles|next_pc|next_npc|delay|result_flags_pairs (hex)"]
    starts = [0x100, 0xFFF, 0x1FFF, 0x10FF]
    for base in (0xC00, 0xC40, 0xC80, 0xCC0, 0xD00, 0xD40, 0xD80, 0xDC0):
        for target in range(4):
            for initial_flags in range(16):
                start = starts[initial_flags & 3]
                cpu = machine(namespace, tables, [start, 0x1A00, 3, 5, 0x20, 0x21, 0xA5, initial_flags])
                for initial_value in range(16):
                    outcomes = []
                    metadata = None
                    for immediate in range(16):
                        cpu._A, cpu._B = 3, 5
                        cpu._PC, cpu._NPC = start, 0x1A00
                        cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (
                            bool(initial_flags & (1 << bit)) for bit in range(4))
                        cpu._RAM = [(i * 3 + 1) & 15 for i in range(0x300)]
                        cpu._VRAM = [(i + 5) & 15 for i in range(0xA0)]
                        cpu._set_abmxmy_tbl[target](cpu, initial_value)
                        before_ram, before_lcd = cpu._RAM.copy(), cpu._VRAM.copy()
                        before_a, before_b = cpu._A, cpu._B
                        # clock() resets this immediately before instruction dispatch.
                        cpu._if_delay = False
                        opcode = base | (target << 4) | immediate
                        cycles = cpu._execute[opcode](cpu, opcode)
                        result = cpu._get_abmxmy_tbl[target](cpu)
                        if target == 2:
                            before_ram[0x20] = result
                        elif target == 3:
                            before_ram[0x21] = result
                        assert cpu._RAM == before_ram and cpu._VRAM == before_lcd
                        assert cpu._A == (result if target == 0 else before_a)
                        assert cpu._B == (result if target == 1 else before_b)
                        assert (cpu._IX, cpu._IY, cpu._SP, cpu._HALT) == (0x20, 0x21, 0xA5, 0)
                        assert (cpu._DF, cpu._IF) == (bool(initial_flags & 4), bool(initial_flags & 8))
                        assert cycles == 7 and not cpu._if_delay
                        current_metadata = (cycles, cpu._PC, cpu._NPC, int(cpu._if_delay))
                        if metadata is not None:
                            assert current_metadata == metadata
                        metadata = current_metadata
                        outcomes.append(f"{result:X}{flags(cpu):X}")
                    lines.append("|".join([
                        *(f"{v:X}" for v in (base, target, initial_flags, initial_value,
                                             cycles, cpu._PC, cpu._NPC, int(cpu._if_delay))),
                        "".join(outcomes)]))
    return lines


def decoder_inventory(namespace, tables, source):
    supported = {
        "_jp_s", "_ld_y_y", "_ld_x_x", "_add_r_i", "_adc_r_i", "_and_r_i",
        "_or_r_i", "_xor_r_i", "_sbc_r_i", "_fan_r_i", "_cp_r_i", "_ld_r_i",
        "_pset_p", "_ldpx_mx_i", "_ldpy_my_i", "_ld_r_q", "_nop5", "_nop7",
        "_add_r_q", "_adc_r_q", "_sub_r_q", "_sbc_r_q", "_and_r_q",
        "_or_r_q", "_xor_r_q", "_cp_r_q", "_fan_r_q",
        "_adc_xh_i", "_adc_xl_i", "_adc_yh_i", "_adc_yl_i",
        "_cp_xh_i", "_cp_xl_i", "_cp_yh_i", "_cp_yl_i",
        "_acpx_mx_r", "_acpy_my_r", "_scpx_mx_r", "_scpy_my_r",
        "_rlc_r", "_rrc_r",
        "_inc_mn", "_dec_mn", "_inc_sp", "_dec_sp",
        "_set_f_i", "_rst_f_i", "_halt",
        "_call_s", "_calz_s", "_ret", "_rets", "_retd_l",
        "_push_r", "_push_xp", "_push_xh", "_push_xl", "_push_yp", "_push_yh", "_push_yl", "_push_f",
        "_pop_r", "_pop_xp", "_pop_xh", "_pop_xl", "_pop_yp", "_pop_yh", "_pop_yl", "_pop_f",
        "_jp_c_s", "_jp_nc_s", "_jp_z_s", "_jp_nz_s", "_jpba",
        "_lbpx_mx_l", "_ld_xp_r", "_ld_xh_r", "_ld_xl_r",
        "_ld_yp_r", "_ld_yh_r", "_ld_yl_r", "_ld_r_xp", "_ld_r_xh",
        "_ld_r_xl", "_ld_r_yp", "_ld_r_yh", "_ld_r_yl",
        "_ldpx_r_q", "_ldpy_r_q", "_ld_mn_a", "_ld_mn_b", "_ld_a_mn", "_ld_b_mn",
        "_ld_sph_r", "_ld_r_sph", "_ld_spl_r", "_ld_r_spl",
    }
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    handlers = [handler.__name__ for handler in cpu._execute]
    assert len(handlers) == 4096
    assert supported == set(handlers) - {"_dummy"}
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# first|last|original_handler|supported (hex)"]
    start = 0
    for end in range(1, len(handlers) + 1):
        if end == len(handlers) or handlers[end] != handlers[start]:
            name = handlers[start]
            lines.append(f"{start:03X}|{end - 1:03X}|{name}|{int(name in supported)}")
            start = end
    return lines


def two_operand_alu_matrix(namespace, tables, source):
    """Exhaustive independent values, coherent aliases, and LCD boundary cases.

    Layouts: 0=distinct RAM, 1=aliased RAM, 2=distinct LCD, 3=aliased LCD.
    Same-storage operands receive the same value, never two incompatible seeds.
    """
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# opcode|layout|initial_flags|left|right_values|cycles|next_pc|next_npc|delay|result_flags_pairs (hex)"]
    ram_seed = [(i * 3 + 1) & 15 for i in range(0x300)]
    lcd_seed = [(i + 5) & 15 for i in range(0xA0)]
    starts = [0x100, 0xFFF, 0x1FFF, 0x10FF]
    cases = 0
    for base in (0xA80, 0xA90, 0xAA0, 0xAB0, 0xAC0, 0xAD0, 0xAE0, 0xF00, 0xF10):
        for target in range(4):
            for source_operand in range(4):
                opcode = base | (target << 2) | source_operand
                for layout, (ix, iy) in enumerate(((0x20, 0x21), (0x20, 0x20),
                                                  (0xE4F, 0xECF), (0xE80, 0xE80))):
                    if layout in (1, 3) and {target, source_operand} != {2, 3}:
                        continue  # Distinct selectors for explicitly aliased M(X)/M(Y).
                    if layout == 2 and target < 2 and source_operand < 2:
                        continue  # Register-only cases are already exhaustive in RAM layout.
                    alias = target == source_operand or (target >= 2 and source_operand >= 2 and ix == iy)
                    values = range(16) if layout < 2 else (0, 1, 9, 15)
                    for initial_flags in range(16):
                        start = starts[initial_flags & 3]
                        cpu = machine(namespace, tables, [start, 0x1A00, 3, 5, ix, iy, 0xA5, initial_flags])
                        for left in values:
                            right_values = (left,) if alias else values
                            outcomes = []
                            metadata = None
                            for right in right_values:
                                cpu._A, cpu._B = 3, 5
                                cpu._PC, cpu._NPC = start, 0x1A00
                                cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (
                                    bool(initial_flags & (1 << bit)) for bit in range(4))
                                cpu._RAM, cpu._VRAM = ram_seed.copy(), lcd_seed.copy()
                                cpu._set_abmxmy_tbl[target](cpu, left)
                                cpu._set_abmxmy_tbl[source_operand](cpu, right)
                                assert cpu._get_abmxmy_tbl[target](cpu) == left
                                assert cpu._get_abmxmy_tbl[source_operand](cpu) == right
                                before_ram, before_lcd = cpu._RAM.copy(), cpu._VRAM.copy()
                                before_a, before_b = cpu._A, cpu._B
                                cpu._if_delay = False
                                cycles = cpu._execute[opcode](cpu, opcode)
                                result = cpu._get_abmxmy_tbl[target](cpu)
                                if target >= 2:
                                    address = ix if target == 2 else iy
                                    if address < 0x300:
                                        before_ram[address] = result
                                    else:
                                        index = address - 0xE00 if address < 0xE50 else address - 0xE80 + 0x50
                                        before_lcd[index] = result
                                assert cpu._A == (result if target == 0 else before_a)
                                assert cpu._B == (result if target == 1 else before_b)
                                assert cpu._RAM == before_ram and cpu._VRAM == before_lcd
                                assert cpu._get_abmxmy_tbl[source_operand](cpu) == (result if alias else right)
                                assert (cpu._IX, cpu._IY, cpu._SP, cpu._HALT) == (ix, iy, 0xA5, 0)
                                assert (cpu._DF, cpu._IF) == (bool(initial_flags & 4), bool(initial_flags & 8))
                                if base in (0xF00, 0xF10):
                                    assert result == left  # CP/FAN never write the operand.
                                assert cycles == 7 and not cpu._if_delay
                                current_metadata = (cycles, cpu._PC, cpu._NPC, int(cpu._if_delay))
                                if metadata is not None:
                                    assert current_metadata == metadata
                                metadata = current_metadata
                                outcomes.append(f"{result:X}{flags(cpu):X}")
                                cases += 1
                            lines.append("|".join([
                                *(f"{v:X}" for v in (opcode, layout, initial_flags, left)),
                                "".join(f"{v:X}" for v in right_values),
                                *(f"{v:X}" for v in metadata), "".join(outcomes)]))
    assert cases == 481536
    return lines, cases


def index_alu_matrix(namespace, tables, source):
    """All index nibbles/immediates/flags in eight preserved-field backgrounds."""
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# base|initial_flags|ix|iy|cycles|next_pc|next_npc|delay|index_flags_groups (hex; 3 index digits + 1 flags digit per immediate)"]
    backgrounds = (0x000, 0x0FF, 0x200, 0x2FF, 0xE00, 0xEFF, 0xF00, 0xFFF)
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    ram_seed = [(i * 3 + 1) & 15 for i in range(0x300)]
    lcd_seed = [(i + 5) & 15 for i in range(0xA0)]
    cases = 0
    for base in range(0xA00, 0xA80, 0x10):
        target = (base >> 4) & 3
        shift = 4 if target in (0, 2) else 0
        field_mask = 15 << shift
        for background in backgrounds:
            for initial_value in range(16):
                address = (background & ~field_mask) | (initial_value << shift)
                ix, iy = (address, 0x321) if target < 2 else (0xABC, address)
                for initial_flags in range(16):
                    start = starts[initial_flags & 3]
                    cpu = machine(namespace, tables, [start, 0x1A00, 3, 5, ix, iy, 0xA5, initial_flags])
                    cpu._RAM, cpu._VRAM = ram_seed.copy(), lcd_seed.copy()
                    outcomes = []
                    metadata = None
                    for immediate in range(16):
                        cpu._IX, cpu._IY = ix, iy
                        cpu._PC, cpu._NPC = start, 0x1A00
                        cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (
                            bool(initial_flags & (1 << bit)) for bit in range(4))
                        cpu._if_delay = False
                        cycles = cpu._execute[base | immediate](cpu, base | immediate)
                        result = cpu._IX if target < 2 else cpu._IY
                        assert (result & ~field_mask) == (address & ~field_mask)
                        assert (cpu._IY if target < 2 else cpu._IX) == (iy if target < 2 else ix)
                        if base >= 0xA40:
                            assert result == address  # Index CP does not change the pointer.
                        assert (cpu._A, cpu._B, cpu._SP, cpu._HALT) == (3, 5, 0xA5, 0)
                        assert (cpu._DF, cpu._IF) == (bool(initial_flags & 4), bool(initial_flags & 8))
                        assert cpu._RAM == ram_seed and cpu._VRAM == lcd_seed
                        assert cycles == 7 and not cpu._if_delay
                        current_metadata = (cycles, cpu._PC, cpu._NPC, int(cpu._if_delay))
                        if metadata is not None:
                            assert current_metadata == metadata
                        metadata = current_metadata
                        outcomes.append(f"{result:03X}{flags(cpu):X}")
                        cases += 1
                    lines.append("|".join([
                        *(f"{v:X}" for v in (base, initial_flags, ix, iy, *metadata)),
                        "".join(outcomes)]))
    assert cases == 262144
    return lines, cases


def post_increment_matrix(namespace, tables, source):
    """Original memory arithmetic with old-pointer reads and page-local increment."""
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# opcode|layout|initial_flags|left|right_values|cycles|next_pc|next_npc|next_ix|next_iy|delay|result_flags_pairs (hex)"]
    layouts = ((0x20, 0x21), (0x20, 0x20), (0x2FF, 0x1FF),
               (0xE4F, 0xECF), (0xE4F, 0xE4F), (0x2FF, 0xE4F), (0xECF, 0x2FF))
    ram_seed = [(i * 3 + 1) & 15 for i in range(0x300)]
    lcd_seed = [(i + 5) & 15 for i in range(0xA0)]
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    cases = 0
    for base in (0xF28, 0xF2C, 0xF38, 0xF3C):
        is_x = base in (0xF28, 0xF38)
        for operand in range(4):
            opcode = base | operand
            for layout, (ix, iy) in enumerate(layouts):
                address = ix if is_x else iy
                alias = operand >= 2 and (ix if operand == 2 else iy) == address
                for initial_flags in range(16):
                    start = starts[initial_flags & 3]
                    cpu = machine(namespace, tables, [start, 0x1A00, 3, 5, ix, iy, 0xA5, initial_flags])
                    for left in range(16):
                        right_values = (left,) if alias else range(16)
                        outcomes = []
                        metadata = None
                        for right in right_values:
                            cpu._A, cpu._B = 3, 5
                            cpu._IX, cpu._IY = ix, iy
                            cpu._PC, cpu._NPC = start, 0x1A00
                            cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (
                                bool(initial_flags & (1 << bit)) for bit in range(4))
                            cpu._RAM, cpu._VRAM = ram_seed.copy(), lcd_seed.copy()
                            cpu.set_mem(address, left)
                            cpu._set_abmxmy_tbl[operand](cpu, right)
                            assert cpu.get_mem(address) == left
                            assert cpu._get_abmxmy_tbl[operand](cpu) == right
                            before_ram, before_lcd = cpu._RAM.copy(), cpu._VRAM.copy()
                            before_a, before_b = cpu._A, cpu._B
                            cpu._if_delay = False
                            cycles = cpu._execute[opcode](cpu, opcode)
                            result = cpu.get_mem(address)  # Read the OLD destination, not M(X/Y) after increment.
                            if address < 0x300:
                                before_ram[address] = result
                            else:
                                index = address - 0xE00 if address < 0xE50 else address - 0xE80 + 0x50
                                before_lcd[index] = result
                            assert cpu._RAM == before_ram and cpu._VRAM == before_lcd
                            assert (cpu._A, cpu._B, cpu._SP, cpu._HALT) == (before_a, before_b, 0xA5, 0)
                            assert (cpu._IY if is_x else cpu._IX) == (iy if is_x else ix)
                            assert ((cpu._IX if is_x else cpu._IY) & 0xF00) == (address & 0xF00)
                            assert (cpu._DF, cpu._IF) == (bool(initial_flags & 4), bool(initial_flags & 8))
                            assert cycles == 7 and not cpu._if_delay
                            current = (cycles, cpu._PC, cpu._NPC, cpu._IX, cpu._IY, int(cpu._if_delay))
                            if metadata is not None:
                                assert current == metadata
                            metadata = current
                            outcomes.append(f"{result:X}{flags(cpu):X}")
                            cases += 1
                        lines.append("|".join([
                            *(f"{v:X}" for v in (opcode, layout, initial_flags, left)),
                            "".join(f"{v:X}" for v in right_values),
                            *(f"{v:X}" for v in metadata), "".join(outcomes)]))
    assert cases == 320512
    return lines, cases


def rotation_matrix(namespace, tables, source):
    """All reference rotation encodings, values, flags, and memory layouts."""
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# opcode|layout|initial_flags|cycles|next_pc|next_npc|delay|result_flags_pairs (hex; 16 input values)"]
    layouts = ((0x20, 0x21), (0x20, 0x20), (0x2FF, 0),
               (0xE00, 0xE4F), (0xE80, 0xECF), (0xE4F, 0xE4F))
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    ram_seed = [(i * 3 + 1) & 15 for i in range(0x300)]
    lcd_seed = [(i + 5) & 15 for i in range(0xA0)]
    cases = 0
    for opcode in (*range(0xAF0, 0xB00), *range(0xE8C, 0xE90)):
        operand = opcode & 3
        for layout, (ix, iy) in enumerate(layouts):
            for initial_flags in range(16):
                start = starts[initial_flags & 3]
                cpu = machine(namespace, tables, [start, 0x1A00, 3, 5, ix, iy, 0xA5, initial_flags])
                metadata = None
                outcomes = []
                for value in range(16):
                    cpu._A, cpu._B = 3, 5
                    cpu._PC, cpu._NPC = start, 0x1A00
                    cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (
                        bool(initial_flags & (1 << bit)) for bit in range(4))
                    cpu._RAM, cpu._VRAM = ram_seed.copy(), lcd_seed.copy()
                    cpu._set_abmxmy_tbl[operand](cpu, value)
                    before_ram, before_lcd = cpu._RAM.copy(), cpu._VRAM.copy()
                    before_a, before_b = cpu._A, cpu._B
                    cpu._if_delay = False
                    cycles = cpu._execute[opcode](cpu, opcode)
                    result = cpu._get_abmxmy_tbl[operand](cpu)
                    if operand >= 2:
                        address = ix if operand == 2 else iy
                        if address < 0x300:
                            before_ram[address] = result
                        else:
                            index = address - 0xE00 if address < 0xE50 else address - 0xE80 + 0x50
                            before_lcd[index] = result
                    assert cpu._RAM == before_ram and cpu._VRAM == before_lcd
                    assert cpu._A == (result if operand == 0 else before_a)
                    assert cpu._B == (result if operand == 1 else before_b)
                    assert (cpu._IX, cpu._IY, cpu._SP, cpu._HALT) == (ix, iy, 0xA5, 0)
                    assert flags(cpu) & 0xE == initial_flags & 0xE  # Preserve Z, D, I, even for result zero.
                    assert cycles == (7 if opcode < 0xB00 else 5)
                    assert not cpu._if_delay
                    current = (cycles, cpu._PC, cpu._NPC, int(cpu._if_delay))
                    if metadata is not None:
                        assert current == metadata
                    metadata = current
                    outcomes.append(f"{result:X}{flags(cpu):X}")
                    cases += 1
                lines.append("|".join([
                    *(f"{v:X}" for v in (opcode, layout, initial_flags, *metadata)),
                    "".join(outcomes)]))
    assert cases == 30720
    return lines, cases


def inc_dec_matrix(namespace, tables, source):
    """All direct nibble-RAM values and all eight-bit SP values with every flag combination."""
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# opcode|initial_flags|cycles|next_pc|next_npc|delay|result_flags_groups (hex; 2 result digits + 1 flags digit per input)"]
    ram_seed = [(i * 3 + 1) & 15 for i in range(0x300)]
    lcd_seed = [(i + 5) & 15 for i in range(0xA0)]
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    cases = [0, 0]
    for opcode in (*range(0xF60, 0xF80), 0xFCB, 0xFDB):
        is_sp = opcode in (0xFCB, 0xFDB)
        for initial_flags in range(16):
            start = starts[initial_flags & 3]
            # Invalid IX/IY must never be dereferenced by these direct/SP instructions.
            cpu = machine(namespace, tables, [start, 0x1A00, 3, 5, 0xF00, 0xE50, 0xA5, initial_flags])
            outcomes = []
            metadata = None
            for value in range(256 if is_sp else 16):
                cpu._PC, cpu._NPC = start, 0x1A00
                cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (
                    bool(initial_flags & (1 << bit)) for bit in range(4))
                cpu._SP = value if is_sp else 0xA5
                cpu._RAM, cpu._VRAM = ram_seed.copy(), lcd_seed.copy()
                if not is_sp:
                    cpu.set_mem(opcode & 15, value)
                expected_ram = cpu._RAM.copy()
                cpu._if_delay = False
                cycles = cpu._execute[opcode](cpu, opcode)
                result = cpu._SP if is_sp else cpu.get_mem(opcode & 15)
                if is_sp:
                    assert flags(cpu) == initial_flags  # SP wrapping changes no flags.
                else:
                    expected_ram[opcode & 15] = result
                    assert cpu._SP == 0xA5
                assert cpu._RAM == expected_ram and cpu._VRAM == lcd_seed
                assert (cpu._A, cpu._B, cpu._IX, cpu._IY, cpu._HALT) == (3, 5, 0xF00, 0xE50, 0)
                assert (cpu._DF, cpu._IF) == (bool(initial_flags & 4), bool(initial_flags & 8))
                assert cycles == (5 if is_sp else 7)
                assert not cpu._if_delay
                current = (cycles, cpu._PC, cpu._NPC, int(cpu._if_delay))
                if metadata is not None:
                    assert current == metadata
                metadata = current
                outcomes.append(f"{result:02X}{flags(cpu):X}")
                cases[int(is_sp)] += 1
            lines.append("|".join([
                *(f"{v:X}" for v in (opcode, initial_flags, *metadata)),
                "".join(outcomes)]))
    assert cases == [8192, 8192]
    return lines, sum(cases)


def flag_control_matrix(namespace, tables, source):
    """Every SET/RST encoding, input flag combination, incoming delay, and PC layout."""
    lines = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# opcode|initial_flags|initial_delay|pc|cycles|flags|delay|next_pc|next_npc (hex)"]
    ram_seed = [(i * 3 + 1) & 15 for i in range(0x300)]
    lcd_seed = [(i + 5) & 15 for i in range(0xA0)]
    cases = 0
    for opcode in range(0xF40, 0xF60):
        for initial_flags in range(16):
            for initial_delay in range(2):
                for pc in (0x100, 0xFFF, 0x1FFF, 0x10FF):
                    # Invalid indirect addresses must not be accessed by flag control.
                    cpu = machine(namespace, tables, [pc, 0x1A00, 3, 5, 0xF00, 0xE50, 0xA5, initial_flags])
                    cpu._RAM, cpu._VRAM = ram_seed.copy(), lcd_seed.copy()
                    cpu._if_delay = bool(initial_delay)
                    # clock() clears the old delay before dispatch; do not execute IRQs.
                    cpu._if_delay = False
                    cycles = cpu._execute[opcode](cpu, opcode)
                    assert cycles == 7
                    assert (cpu._A, cpu._B, cpu._IX, cpu._IY, cpu._SP, cpu._HALT) == (
                        3, 5, 0xF00, 0xE50, 0xA5, 0)
                    assert cpu._RAM == ram_seed and cpu._VRAM == lcd_seed
                    lines.append("|".join(f"{int(v):X}" for v in (
                        opcode, initial_flags, initial_delay, pc, cycles, flags(cpu),
                        cpu._if_delay, cpu._PC, cpu._NPC)))
                    cases += 1
    assert cases == 4096
    return lines, cases


def extension_state(cpu):
    return [cpu._PC, cpu._NPC, cpu._A, cpu._B, cpu._IX, cpu._IY,
            cpu._SP, flags(cpu), int(cpu._if_delay), cpu._HALT]


def extension_case(cpu, group, opcode, initial, seed, ram_patch=None):
    """Run the original handler and encode the full state plus complete memory differences."""
    for name, value in zip(("_PC", "_NPC", "_A", "_B", "_IX", "_IY", "_SP"), initial[:7]):
        setattr(cpu, name, value)
    cpu._CF, cpu._ZF, cpu._DF, cpu._IF = (bool(initial[7] & (1 << bit)) for bit in range(4))
    cpu._if_delay, cpu._HALT = bool(initial[8]), initial[9]
    cpu._RAM = [(i * 3 + seed) & 15 for i in range(0x300)]
    cpu._VRAM = [(i + seed + 4) & 15 for i in range(0xA0)]
    ram_patch = ram_patch or {}
    for address, value in ram_patch.items():
        cpu._RAM[address] = value
    before_ram, before_lcd = cpu._RAM.copy(), cpu._VRAM.copy()
    assert not cpu._HALT
    assert cpu._execute[opcode].__name__ != "_dummy"
    # Match clock() instruction dispatch, excluding IRQ acceptance and MCU clocks.
    cpu._if_delay = False
    cycles = cpu._execute[opcode](cpu, opcode)

    def values(items):
        return ",".join(f"{int(v):X}" for v in items)

    def differences(before, after):
        return ",".join(f"{i:X}:{b:X}" for i, (a, b) in enumerate(zip(before, after)) if a != b)

    return "|".join([group, f"{opcode:X}", f"{seed:X}", values(initial),
                     ",".join(f"{i:X}:{v:X}" for i, v in sorted(ram_patch.items())),
                     f"{cycles:X}", values(extension_state(cpu)),
                     differences(before_ram, cpu._RAM), differences(before_lcd, cpu._VRAM)])


def extension_header(source):
    return ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
            "# group|opcode|seed|initial_state|initial_ram_patch|cycles|final_state|ram_changes|lcd_changes",
            "# state: pc,npc,a,b,ix,iy,sp,flags,delay,halt (hex); memory changes include zero values"]


def load_matrix(namespace, tables, source):
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    lines = extension_header(source)
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    for opcode in range(0x900, 0xA00):
        for initial_flags in range(16):
            for ix in (0, 0xFF, 0x2FE, 0x2FF, 0xE00, 0xE4E, 0xE80, 0xECE):
                state = [starts[initial_flags & 3], 0x1A00, 3, 5, ix, 0x21,
                         0xA5, initial_flags, 1, 0]
                lines.append(extension_case(cpu, "loads", opcode, state, initial_flags))
    opcodes = [*range(0xE80, 0xE8C), *range(0xE90, 0xE9C),
               *range(0xEA0, 0xEAC), *range(0xEB0, 0xEBC),
               *range(0xEE0, 0xF00), *range(0xF80, 0xFC0),
               *range(0xFE0, 0xFE8), *range(0xFF0, 0xFF8)]
    inputs = [(f, v) for f in (0, 15) for v in range(16)]
    inputs += [(f, 0) for f in range(1, 15)]
    for opcode in opcodes:
        for initial_flags, seed in inputs:
            for ix, iy in ((0x20, 0x21), (0x20, 0x20), (0x2FF, 0x1FF),
                           (0xE4F, 0xECF), (0xE80, 0xE80), (0, 0xFF)):
                state = [starts[(seed + initial_flags) & 3], 0x1A00, seed, 15 - seed,
                         ix, iy, (seed << 4) | (15 - seed), initial_flags, 1, 0]
                lines.append(extension_case(cpu, "loads", opcode, state, seed))
    assert len(lines) - 3 == 76928
    return lines


def branch_matrix(namespace, tables, source):
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    lines = extension_header(source)
    contexts = ((0x100, 0x100), (0xFFF, 0x1F00), (0x1FFF, 0), (0x10FF, 0x1A55))
    for base in (0, 0x200, 0x300, 0x600, 0x700):
        for target in range(256):
            for initial_flags in range(16):
                for pc, npc in contexts:
                    state = [pc, npc, 3, 5, 0xF00, 0xE50, 0xA5, initial_flags, 1, 0]
                    lines.append(extension_case(cpu, "branches", base | target, state, initial_flags))
    for a in range(16):
        for b in range(16):
            for initial_flags in range(16):
                for pc, npc in contexts:
                    state = [pc, npc, a, b, 0xF00, 0xE50, 0xA5, initial_flags, 1, 0]
                    lines.append(extension_case(cpu, "branches", 0xFE8, state, initial_flags))
    assert len(lines) - 3 == 98304
    return lines


def stack_matrix(namespace, tables, source):
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    lines = extension_header(source)
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    for opcode in (*range(0xFC0, 0xFCB), *range(0xFD0, 0xFDB)):
        for sp in range(256):
            for initial_flags in (0, 15):
                seed = sp & 15
                state = [starts[sp & 3], 0x1A55, seed, 15 - seed, 0x20, 0x21,
                         sp, initial_flags, 1, 0]
                lines.append(extension_case(cpu, "stack", opcode, state, seed))
        for sp in (0, 1, 2, 0xFD, 0xFE, 0xFF):
            for initial_flags in range(16):
                for seed in range(16):
                    alias = ((sp - 1) & 255) if opcode < 0xFD0 else sp
                    for layout, (ix, iy) in enumerate(((0x20, 0x21), (alias, alias), (0xE4F, 0xECF))):
                        # Register-only variants can exercise every nibble without memory access.
                        if layout == 0 and 4 <= (opcode & 15) <= 9:
                            ix, iy = seed * 0x111, (15 - seed) * 0x111
                        state = [starts[(seed + initial_flags) & 3], 0x1A55,
                                 seed, 15 - seed, ix, iy, sp, initial_flags, 1, 0]
                        lines.append(extension_case(cpu, "stack", opcode, state, seed))
    assert len(lines) - 3 == 112640
    return lines


def subroutine_matrix(namespace, tables, source):
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    lines = extension_header(source)
    contexts = ((0x100, 0x100), (0xFFF, 0x1F00), (0x1FFF, 0), (0x10FF, 0x1A55))
    boundaries = (0, 1, 2, 0xFD, 0xFE, 0xFF)
    for base in (0x400, 0x500):
        for target in range(256):
            for pc, npc in contexts:
                for sp in boundaries:
                    for initial_flags in (0, 15):
                        state = [pc, npc, 3, 5, 0x20, 0x21, sp, initial_flags, 1, 0]
                        lines.append(extension_case(cpu, "subroutines", base | target, state, target & 15))
        for target in (0, 255):
            for sp in range(256):
                for initial_flags in range(16):
                    pc, npc = contexts[sp & 3]
                    state = [pc, npc, 3, 5, 0x20, 0x21, sp, initial_flags, 1, 0]
                    lines.append(extension_case(cpu, "subroutines", base | target, state, sp & 15))
    for opcode in (0xFDE, 0xFDF):
        for address in range(0x1000):
            sp = address & 255
            patch = {sp: address & 15, sp + 1: (address >> 4) & 15, sp + 2: address >> 8}
            for bank in (0, 0x1000):
                for initial_flags in (0, 15):
                    state = [bank | 0x100, 0x1A55, 3, 5, 0x20, 0x21, sp, initial_flags, 1, 0]
                    lines.append(extension_case(cpu, "subroutines", opcode, state, address & 15, patch))
    for value in range(256):
        for sp in boundaries:
            for ix in (0x2FF, 0xE4E, sp, sp + 1):
                for initial_flags in (0, 15):
                    pc, npc = contexts[value & 3]
                    patch = {sp: value & 15, sp + 1: (value >> 4) & 15, sp + 2: (value * 7) & 15}
                    state = [pc, npc, 3, 5, ix, 0x21, sp, initial_flags, 1, 0]
                    lines.append(extension_case(cpu, "subroutines", 0x100 | value, state, value & 15, patch))
    for initial_flags in range(16):
        for seed in range(16):
            for sp in boundaries:
                for ix in (0xFF, 0xECE, sp, sp + 1):
                    pc, npc = contexts[seed & 3]
                    patch = {sp: seed, sp + 1: (seed + 1) & 15, sp + 2: (seed + 2) & 15}
                    state = [pc, npc, 3, 5, ix, 0x21, sp, initial_flags, 1, 0]
                    lines.append(extension_case(cpu, "subroutines", 0x1AB, state, seed, patch))
    assert len(lines) - 3 == 92160
    return lines


def control_matrix(namespace, tables, source):
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    lines = extension_header(source)
    starts = (0x100, 0xFFF, 0x1FFF, 0x10FF)
    for opcode in (0xFF8, 0xFFB, 0xFFF):
        for initial_flags in range(16):
            for pc in starts:
                for delay in range(2):
                    for ix, iy in ((0xF00, 0xE50), (0x2FF, 0xECF)):
                        state = [pc, 0x1A55, 3, 5, ix, iy, 0xFF, initial_flags, delay, 0]
                        lines.append(extension_case(cpu, "control", opcode, state, initial_flags))
    for opcode in range(0xE40, 0xE60):
        for initial_flags in range(16):
            for pc in starts:
                for delay in range(2):
                    state = [pc, 0x1A55, 3, 5, 0xF00, 0xE50, 0xFF, initial_flags, delay, 0]
                    lines.append(extension_case(cpu, "control", opcode, state, initial_flags))
    assert len(lines) - 3 == 4864
    return lines


def emit(name, lines, check):
    path = DATA / name
    content = "\n".join(lines) + "\n"
    if check:
        if path.read_text(encoding="utf-8") != content:
            raise SystemExit(f"Reference fixture differs: {path}")
    else:
        path.write_text(content, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify fixtures without writing")
    args = parser.parse_args()
    namespace, tables, source = reference_class()
    lines = ["# Generated from E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
             "# case|step|cycles|a|b|ix|iy|sp|pc|npc|flags|delay|halt|RAM|LCD (hex)"]
    count = 0
    programs = 0
    for line in (DATA / "e0c6200_programs.txt").read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        fields = line.split("|")
        programs += 1
        cpu = machine(namespace, tables, [int(v, 16) for v in fields[1:9]])
        program = dict(tuple(int(v, 16) for v in pair.split(":"))
                       for pair in fields[10].split(","))
        for step in range(int(fields[9])):
            assert not cpu._HALT, f"Synthetic program {fields[0]} attempts dispatch after HALT"
            opcode = program[cpu._PC]
            # Same instruction dispatch and delay reset as clock(), without MCU clocks/IRQs.
            cpu._if_delay = False
            cycles = cpu._execute[opcode](cpu, opcode)
            values = [step, cycles, cpu._A, cpu._B, cpu._IX, cpu._IY, cpu._SP,
                      cpu._PC, cpu._NPC, flags(cpu), int(cpu._if_delay), cpu._HALT]
            lines.append("|".join([fields[0], *(f"{int(v):X}" for v in values),
                                   sparse(cpu._RAM), sparse(cpu._VRAM)]))
            count += 1
    emit("e0c6200_reference.txt", lines, args.check)

    arithmetic = ["# E0C6200.py SHA256 " + hashlib.sha256(source).hexdigest(),
                  "# decimal|initial_A|immediate|result_A|carry|zero (hex)"]
    for decimal in range(2):
        for a in range(16):
            for immediate in range(16):
                cpu = machine(namespace, tables, [0x100, 0x100, a, 0, 0, 0, 0, decimal << 2])
                opcode = 0xC00 | immediate
                assert cpu._execute[opcode](cpu, opcode) == 7
                arithmetic.append("|".join(f"{int(v):X}" for v in
                                          (decimal, a, immediate, cpu._A, cpu._CF, cpu._ZF)))
    emit("e0c6200_add_reference.txt", arithmetic, args.check)
    emit("e0c6200_immediate_alu_reference.txt",
         immediate_alu_matrix(namespace, tables, source), args.check)
    emit("e0c6200_decode_reference.txt", decoder_inventory(namespace, tables, source), args.check)
    two_operand, two_operand_cases = two_operand_alu_matrix(namespace, tables, source)
    emit("e0c6200_two_operand_alu_reference.txt", two_operand, args.check)
    index, index_cases = index_alu_matrix(namespace, tables, source)
    emit("e0c6200_index_alu_reference.txt", index, args.check)
    post_increment, post_cases = post_increment_matrix(namespace, tables, source)
    emit("e0c6200_post_increment_reference.txt", post_increment, args.check)
    rotations, rotation_cases = rotation_matrix(namespace, tables, source)
    emit("e0c6200_rotation_reference.txt", rotations, args.check)
    inc_dec, inc_dec_cases = inc_dec_matrix(namespace, tables, source)
    emit("e0c6200_inc_dec_reference.txt", inc_dec, args.check)
    flag_control, flag_cases = flag_control_matrix(namespace, tables, source)
    emit("e0c6200_flag_reference.txt", flag_control, args.check)
    extensions = (("loads", load_matrix), ("branches", branch_matrix), ("stack", stack_matrix),
                  ("subroutines", subroutine_matrix), ("control", control_matrix))
    for name, generate in extensions:
        rows = generate(namespace, tables, source)
        emit(f"e0c6200_{name}_reference.txt", rows, args.check)
        print(f"{'Verified' if args.check else 'Generated'} {len(rows) - 3} direct {name} cases.")
    action = "Verified" if args.check else "Generated"
    print(f"{action} {count} instruction snapshots from {programs} synthetic programs, "
          "512 legacy ADD cases, 131072 direct immediate ALU cases and "
          f"{two_operand_cases} direct two-operand ALU cases, {index_cases} direct index cases and "
          f"{post_cases} direct post-increment cases, {rotation_cases} direct rotation cases and "
          f"{inc_dec_cases} direct INC/DEC cases and {flag_cases} direct flag-control cases.")


if __name__ == "__main__":
    main()
