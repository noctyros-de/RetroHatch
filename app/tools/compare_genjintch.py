# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Generate/check focused Genjintch MCU fixtures using unchanged original handlers.

The fixtures contain synthetic programs only. Optional --rom and --trace compare
local user-owned firmware against a Rust device_trace run without distributing it.
"""
import sys
sys.dont_write_bytecode = True

import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
from workspace_paths import references_dir
REFERENCES = references_dir()
from compare_e0c6200_mcu import load_reference, scenarios, sparse, CPU, SOUND

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "app/tests/data"
MASTER_HZ = 102400000
PROFILE = json.loads((REFERENCES / "BrickEmuPy-main/assets/TamagotchiGenjintch.brick").read_text(encoding="utf-8"))
assert PROFILE["core"] == "E0C6200" and PROFILE["RAM"] == 768
assert PROFILE["mask_options"]["port_pullup"] == {"K0": 15, "K1": 15}
assert PROFILE["mask_options"]["p3_dedicated"] == 0
assert PROFILE["peripherals"]["direct_input"] == {
    name: {"port": "K0", "mask": mask, "level": 0}
    for name, mask in (("screenFrame", 8), ("btnLeft", 4), ("btnCenter", 2), ("btnRight", 1))}
CLOCK = PROFILE["clock"]
assert CLOCK == 1060000

def state(cpu):
    flags = sum(int(bool(getattr(cpu, attr))) << bit for bit, attr in enumerate(("_CF", "_ZF", "_DF", "_IF")))
    return [cpu._PC, cpu._NPC, cpu._A, cpu._B, cpu._IX, cpu._IY, cpu._SP, flags, int(cpu._if_delay), cpu._HALT]

def gen_scenarios():
    name, clocks, program, pins, writes = scenarios()[0]
    # Enable all four K0 inputs; the rest is the existing real CPU/IRQ/LCD scenario.
    program = dict(program)
    program[0x12B] = 0xE2F
    pins = pins + [(180, 0, 8, 0), (190, 0, 4, 0), (200, 0, 8, -1), (210, 0, 4, -1)]
    yield "genjintch_inputs", 260, program, pins, writes
    name, clocks, program, pins, writes = scenarios()[2]
    yield "genjintch_oscillators", 1200, program, pins, writes + [(400, 0xF70, 0), (640, 0xF70, 8)]
    # Real stores and reads at both LCD gap boundaries, F7C and mapped LCD memory.
    program = dict(enumerate([0xE0E, 0xB4F, 0xE2F, 0xB50, 0xE2A, 0xEC2,
                              0xB7F, 0xE25, 0xB80, 0xE23, 0xBCF, 0xE27,
                              0xBD0, 0xE29, 0xBFF, 0xE26, 0xE0F, 0xB7C,
                              0xE2F, 0xEC2, 0xFF8], 0x100))
    yield "genjintch_disconnected_bus", 80, program, [], []

def generate():
    digest = hashlib.sha256(CPU.read_bytes() + SOUND.read_bytes()).hexdigest()
    inputs = ["# Synthetic Genjintch programs; reference .brick clock=1060000; no firmware"]
    outputs = ["# Original CPU+sound SHA256 " + digest,
               "# Same complete-state schema as e0c6200_mcu_reference.txt"]
    count = 0
    for name, clocks, program, pins, writes in gen_scenarios():
        inputs.append("|".join([name, str(clocks), ",".join(f"{a:X}:{v:X}" for a,v in sorted(program.items())),
            ",".join(f"{s}@{p:X}@{m:X}@{v}" for s,p,m,v in pins),
            ",".join(f"{s}@{a:X}@{v:X}" for s,a,v in writes)]))
        words = [0xFFB] * 0x2000
        for address, value in program.items(): words[address] = value
        cpu, recorder = load_reference(words, CLOCK)
        elapsed = Fraction()
        for clock in range(clocks):
            for step, port, mask, level in pins:
                if step == clock: cpu.pin_handler(("K0", "K1")[port], mask, level)
            for step, address, value in writes:
                if step == clock: cpu.set_mem(address, value)
            elapsed += Fraction(cpu.clock())
            if clock < 64 or (clock+1) % 32 == 0 or clock+1 == clocks:
                names = ("_IT", "_ISW", "_IPT", "_ISIO", "_IK0", "_IK1")
                saved = [getattr(cpu, a) for a in names]
                io = "".join(f"{cpu.get_mem(0xF00+i):X}" for i in range(0x7F))
                for a,v in zip(names,saved): setattr(cpu,a,v)
                b=cpu._sound
                buzzer=[b._one_shot_counter,b._envelope_step,b._envelope_counter,int(b._sound_on),int(b._envelope_on),recorder.level,recorder.frequency_index]
                ticks = int(elapsed * MASTER_HZ / CLOCK)
                outputs.append("|".join([name,str(clock+1),f"{ticks:X}",",".join(f"{int(v):X}" for v in state(cpu)),
                    sparse(cpu._RAM),sparse(cpu._VRAM),io,",".join(f"{int(v):X}" for v in buzzer)]))
                count += 1
    return inputs, outputs, count

def compare_rom(rom, trace, *, clock=CLOCK, rom_bytes=16384):
    data=rom.read_bytes()
    assert len(data)==rom_bytes and all(data[i] < 16 for i in range(0,len(data),2)), f"Expected a {rom_bytes}-byte unpacked E0C6200 image"
    words=[int.from_bytes(data[i:i+2],"big") for i in range(0,len(data),2)]
    cpu, _=load_reference(words,clock)
    elapsed=Fraction(); clocks=0; checkpoints=0; events=0; displays=set()
    for line in trace.read_text(encoding="utf-8").splitlines():
        fields=line.split('|')
        if fields[0] not in ('S','E'): continue
        target=int(fields[1])
        while clocks<target:
            elapsed += Fraction(cpu.clock()); clocks+=1
        assert clocks==target
        if fields[0]=='E':
            cpu.pin_handler('K0',int(fields[2]),0 if fields[3]=='1' else -1)
            events+=1; continue
        context=f"clock {clocks}"
        assert int(fields[2])==int(elapsed*MASTER_HZ/clock),(context,'time')
        assert [int(v,16) for v in fields[3].split(',')]==state(cpu),(context,'CPU')
        assert fields[4]==''.join(f'{v:X}' for v in cpu._RAM),(context,'RAM')
        assert fields[5]==''.join(f'{v:X}' for v in cpu._VRAM),(context,'LCD')
        assert fields[6]==f'{cpu.get_mem(0xF70):X},{cpu.get_mem(0xF71):X}',(context,'oscillator/display control')
        displays.add(fields[5]); checkpoints+=1
    print(f"Verified user ROM SHA256 {hashlib.sha256(data).hexdigest()}: {checkpoints} complete CPU/RAM/LCD checkpoints, {events} input transitions, {clocks} original clocks, {len(displays)} distinct LCD states, {float(elapsed/clock):.6f} seconds.")

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--check',action='store_true')
    p.add_argument('--rom',type=Path); p.add_argument('--trace',type=Path)
    args=p.parse_args()
    if args.rom or args.trace:
        assert args.rom and args.trace,'--rom and --trace are required together'
        compare_rom(args.rom,args.trace)
    else:
        inputs,outputs,count=generate()
        for name,lines in [('genjintch_programs.txt',inputs),('genjintch_reference.txt',outputs)]:
            path=DATA/name; text='\n'.join(lines)+'\n'
            if args.check: assert path.read_text(encoding='utf-8')==text,f'Fixture differs: {path}'
            else: path.write_text(text,encoding='utf-8')
        print(f"{'Verified' if args.check else 'Generated'} {count} complete Genjintch checkpoints across 1540 clocks.")
