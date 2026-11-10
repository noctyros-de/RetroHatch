# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Check the Umino profile and shared synthetic hardware oracle; optionally compare local firmware.

Umino and Genjintch have identical CPU/clock/mask/input configurations. The
existing compact MCU oracle is shared intentionally; no firmware is embedded.
"""
import sys
sys.dont_write_bytecode = True
import argparse
import json
from pathlib import Path
from compare_genjintch import ROOT, DATA, PROFILE, generate, compare_rom
import compare_genjintch as shared

UMINO = json.loads((ROOT / "references/BrickEmuPy-main/assets/TamagotchiUmino.brick").read_text(encoding="utf-8"))
for key in ("core", "clock", "RAM", "peripherals", "buttons"):
    assert UMINO[key] == PROFILE[key], f"Umino hardware differs: {key}"
for key in ("port_pullup", "p3_dedicated"):
    assert UMINO["mask_options"][key] == PROFILE["mask_options"][key], key
assert UMINO["clock"] == 1060000

def generate_bus():
    # Real stores followed by reads at disconnected boundaries, including the
    # startup FD0 write observed in the user ROM. No peripheral is synthesized.
    words=[]
    for address in (0x300,0xDFF,0xE50,0xE7F,0xED0,0xEFF,0xF06,0xF7C,0xF7F,0xFD0,0xFFF):
        words += [0xE00 | (address>>8),0xE80,0xB00 | (address&255),0xE2F,0xEC2]
    words.append(0xFF8)
    scenario=("umino_unmapped_bus",80,dict(enumerate(words,0x100)),[],[])
    old=shared.gen_scenarios
    try:
        shared.gen_scenarios=lambda:iter([scenario])
        inputs,outputs,count=generate()
    finally:
        shared.gen_scenarios=old
    inputs[0]="# Synthetic Umino disconnected-bus program; reference clock=1060000; no firmware"
    return inputs,outputs,count

if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check",action="store_true")
    parser.add_argument("--rom",type=Path)
    parser.add_argument("--trace",type=Path)
    args=parser.parse_args()
    if args.rom or args.trace:
        assert args.rom and args.trace,"--rom and --trace must be supplied together"
        compare_rom(args.rom,args.trace)
    else:
        inputs,outputs,bus_count=generate_bus()
        for name,lines in (("umino_programs.txt",inputs),("umino_reference.txt",outputs)):
            path=DATA/name;expected="\n".join(lines)+"\n"
            if args.check:assert path.read_text(encoding="utf-8")==expected,name
            else:path.write_text(expected,encoding="utf-8")
        print(f"{'Verified' if args.check else 'Generated'} {bus_count} complete Umino disconnected-bus checkpoints.")
        inputs,outputs,count=generate()
        for name,lines in (("genjintch_programs.txt",inputs),("genjintch_reference.txt",outputs)):
            assert (DATA/name).read_text(encoding="utf-8") == "\n".join(lines)+"\n", name
        print(f"Verified Umino .brick clock/masks/GPIO and all {count} shared complete MCU checkpoints across 1540 clocks.")
