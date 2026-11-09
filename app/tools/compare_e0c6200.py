"""Generate reproducible CPU-only fixtures from unchanged BrickEmuPy handlers.

Uses only synthetic instructions; no firmware, imports, constructor, files in
references/, peripherals, clocks or GUI. Run with Python 3 from any directory.
The Rust unit tests consume the checked-in results without requiring Python.
"""

import argparse
import ast
import hashlib
from pathlib import Path

APP = Path(__file__).resolve().parents[1]
SOURCE = APP.parent / "references/BrickEmuPy-main/cores/E0C6200.py"
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
    }
    cpu = machine(namespace, tables, [0x100, 0x100, 0, 0, 0, 0, 0, 0])
    handlers = [handler.__name__ for handler in cpu._execute]
    assert len(handlers) == 4096
    assert supported <= set(handlers)
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
    action = "Verified" if args.check else "Generated"
    print(f"{action} {count} instruction snapshots from {programs} synthetic programs, "
          "512 legacy ADD cases, 131072 direct immediate ALU cases and "
          f"{two_operand_cases} direct two-operand ALU cases, {index_cases} direct index cases and "
          f"{post_cases} direct post-increment cases and {rotation_cases} direct rotation cases.")


if __name__ == "__main__":
    main()
