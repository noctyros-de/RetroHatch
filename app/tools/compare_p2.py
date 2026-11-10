# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Verify the P2/P1 hardware equivalence and the shared original MCU oracle.

Optional --rom/--trace checks user-provided P2 firmware; no ROM is included.
"""
import sys
sys.dont_write_bytecode = True
import argparse
import json
from pathlib import Path
from compare_e0c6200_mcu import DATA, generate
from compare_genjintch import ROOT, compare_rom

ASSETS = ROOT / "references/BrickEmuPy-main/assets"
P1 = json.loads((ASSETS / "TamagotchiP1.brick").read_text(encoding="utf-8"))
P2 = json.loads((ASSETS / "TamagotchiP2.brick").read_text(encoding="utf-8"))
for p in (P1, P2):
    del p["id"]
    del p["mask_options"]["rom_path"]
assert P1 == P2, "P2 hardware/face differs from P1; sharing must be reviewed"
assert P2["clock"] == 1_600_000 and P2["face_path"] == "./assets/TamagotchiP1.svg"
assert P2["peripherals"]["direct_input"] == {
    name: {"port": "K0", "mask": mask, "level": 0}
    for name, mask in (("btnLeft", 4), ("btnCenter", 2), ("btnRight", 1))}

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--rom", type=Path)
    parser.add_argument("--trace", type=Path)
    args = parser.parse_args()
    if args.rom or args.trace:
        assert args.rom and args.trace, "--rom and --trace are required together"
        compare_rom(args.rom, args.trace, clock=P2["clock"], rom_bytes=12_288)
    else:
        inputs, outputs, count = generate()
        for name, lines in (("e0c6200_mcu_programs.txt", inputs), ("e0c6200_mcu_reference.txt", outputs)):
            assert (DATA / name).read_text(encoding="utf-8") == "\n".join(lines) + "\n", name
        print(f"Verified P2/P1 .brick equivalence, shared original SVG and {count} complete MCU oracle checkpoints.")
