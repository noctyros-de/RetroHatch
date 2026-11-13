# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Compare 30 real-firmware checkpoints against T7Emu. Firmware stays local.

The Rust tamago_probe must first write checkpoints.bin with duration 30 and no
figure. C CPU/bus/EEPROM/sound handlers are used with cycle-driven host scheduling;
the upstream Paradise wall-clock RAM synchronization and speed hacks are excluded.
"""
import argparse
from pathlib import Path
import os
import re
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("rom", type=Path)
    parser.add_argument("--reference", type=Path, required=True,
                        help="Paramix-Studio t7 directory; never modified")
    parser.add_argument("--native", type=Path, required=True,
                        help="Private tamago_probe checkpoints.bin")
    args = parser.parse_args()
    tools = Path(__file__).resolve().parent
    source = args.reference / "src/core"
    with tempfile.TemporaryDirectory(prefix="retrohatch-tamago-oracle-") as name:
        work = Path(name)
        for original in source.iterdir():
            if original.suffix not in (".c", ".h", ".inc"):
                continue
            text = original.read_text(encoding="utf-8")
            text = text.replace('__attribute__((always_inline))', '')
            text = text.replace('__attribute__((hot, optimize("O2")))', '')
            if original.name == "t7_cpu.c":
                text = ('static int oracle_clz(unsigned x){int n=0;while(!(x&0x80000000u)){n++;x<<=1;}return n;}\n' +
                        text.replace('__builtin_clz((uint32_t)pending << 16)',
                                     'oracle_clz((uint32_t)pending << 16)'))
            (work / original.name).write_text(text, encoding="utf-8")
        compiler = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
        vs = subprocess.check_output([str(compiler), "-latest", "-products", "*",
            "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], text=True).strip()
        files = [tools / "tamago_reference.c", tools / "tamago_reference_host.c"]
        files += [work / f for f in ["t7.c", "t7_cpu.c", "t7_memory.c", "t7_eeprom.c", "t7_sound.c", "figure_data.c"]]
        cmd = ["cl", "/nologo", "/std:c11", "/O2", "/MT", "/D_CRT_SECURE_NO_WARNINGS",
               "/DT7_VIDEO_CYCLES", "/DT7_STOP", "/DT7_STOP_KEYACT=4000000", "/DT7_PW_FIX",
               "/I" + str(work), *map(str, files), "/Fe:oracle.exe"]
        script = work / "build.cmd"
        script.write_text('@echo off\ncall "' + str(Path(vs)/"Common7/Tools/VsDevCmd.bat") +
                          '" -no_logo -arch=x64 -host_arch=x64\n' + subprocess.list2cmdline(cmd) + '\n')
        subprocess.run(["cmd", "/d", "/c", str(script)], cwd=work, check=True, stdout=subprocess.DEVNULL)
        output = work / "checkpoints.bin"
        subprocess.run([str(work/"oracle.exe"), str(args.rom.resolve()), str(output)], check=True)
        native = args.native.read_bytes()
        expected = output.read_bytes()
        stride = 7 + 1536 + 512 + 256
        assert len(native) == len(expected) == 30 * stride, "Expected 30 checkpoints"
        for index in range(30):
            a, b = native[index*stride:(index+1)*stride], expected[index*stride:(index+1)*stride]
            assert a == b, f"CPU/RAM/LCD/I/O mismatch at second {index+1}"
        print("30 Tama-Go firmware checkpoints match T7Emu CPU, RAM, LCD RAM and I/O exactly.")


if __name__ == "__main__":
    main()
