"""Check the Angel profile and shared synthetic hardware oracle; optionally compare local firmware.

Angel and Genjintch have identical CPU/clock/mask/input configurations. The
existing compact MCU oracle is shared intentionally; no firmware is embedded.
"""
import sys
sys.dont_write_bytecode = True
import argparse
import json
from pathlib import Path
from compare_genjintch import ROOT, DATA, PROFILE, generate, compare_rom

ANGEL = json.loads((ROOT / "references/BrickEmuPy-main/assets/TamagotchiAngel.brick").read_text(encoding="utf-8"))
for key in ("core", "clock", "RAM", "peripherals", "buttons"):
    assert ANGEL[key] == PROFILE[key], f"Angel hardware differs: {key}"
for key in ("port_pullup", "p3_dedicated"):
    assert ANGEL["mask_options"][key] == PROFILE["mask_options"][key], key
assert ANGEL["clock"] == 1060000

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
        inputs,outputs,count=generate()
        for name,lines in (("genjintch_programs.txt",inputs),("genjintch_reference.txt",outputs)):
            assert (DATA/name).read_text(encoding="utf-8") == "\n".join(lines)+"\n", name
        print(f"Verified Angel .brick clock/masks/GPIO and all {count} shared complete MCU checkpoints across 1540 clocks.")
