# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noktyros
"""Compare focused synthetic P1/MCU scenarios with BrickEmuPy's actual clock().

No firmware imports, ROM files, GUI, audio device, or reference files are modified.
The original constructor, registers, handlers, timers, interrupts, and buzzer run.
Only the program-memory and interconnect boundaries are synthetic test adapters.
"""
import argparse
import ast
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "app/tests/data"
CPU = ROOT / "references/BrickEmuPy-main/cores/E0C6200.py"
SOUND = CPU.with_name("E0C6200sound.py")


class SyntheticRom:
    def __init__(self, words):
        self.words = words
    def get_word(self, address):
        return self.words[(address // 2) % len(self.words)]


class Recorder:
    def __init__(self):
        self.level = 0
        self.frequency_index = 0
    def register_pin_device(self, device):
        self.device = device
    def emit_port(self, device, port, value):
        pass  # P1 has no externally connected devices on these output ports.
    def emit_audio(self, channel, tone):
        assert channel == 0
        if tone is None:
            self.level = 0
        else:
            frequency, noise, amplitude, duration = tone
            assert not noise and duration == 0
            self.level = round(amplitude * 7 / 5)
            self.frequency_index = min(range(8), key=lambda i: abs(
                frequency - 32768 / (8, 10, 12, 14, 16, 20, 24, 28)[i]))


def load_reference(words, clock=1600000):
    namespace = {}
    for path in (SOUND, CPU):
        tree = ast.parse(path.read_bytes(), filename=str(path))
        nodes = [node for node in tree.body if not isinstance(node, (ast.Import, ast.ImportFrom))]
        if path == CPU:
            namespace["ROM"] = lambda _: SyntheticRom(words)
        exec(compile(ast.Module(body=nodes, type_ignores=[]), str(path), "exec"), namespace)
    recorder = Recorder()
    cpu = namespace["E0C6200"](
        {"rom_path": None, "port_pullup": {"K0": 15, "K1": 15}, "p3_dedicated": 0},
        clock, recorder)
    return cpu, recorder


def scenarios():
    buttons = {0x100: 0x020}
    main = [0xE0F, 0xE80, 0xB71, 0xE20, 0xE0E, 0xE80, 0xB00, 0xE2F,
            0xE0F, 0xE80, 0xB14, 0xE27, 0xF48, 0xFF8, 0xF48, 0x02D]
    buttons.update(enumerate(main, 0x120))
    handler = [0xE0F, 0xE80, 0xB04, 0xEC2, 0xB40, 0xEC6,
               0xE0E, 0xE80, 0xB10, 0xEC9, 0xFDF]
    buttons.update(enumerate(handler, 0x106))
    pins = [(30, 0, 4, 0), (50, 0, 4, -1), (70, 0, 2, 0), (90, 0, 2, -1),
            (110, 0, 1, 0), (130, 0, 1, -1), (150, 0, 4, 0), (150, 0, 1, 0),
            (170, 0, 4, -1), (170, 0, 1, -1)]

    timers = {0x100: 0x020}
    setup = []
    for register, value in ((0x71, 0), (0x74, 3), (0x75, 1), (0x54, 0),
                            (0x77, 3), (0x26, 2), (0x27, 0), (0x78, 3),
                            (0x79, 15), (0x10, 1), (0x11, 3), (0x12, 0)):
        setup += [0xE0F, 0xE80, 0xB00 | register, 0xE20 | value]
    halt_at = 0x120 + len(setup) + 1
    setup += [0xF48, 0xFF8, 0xF48, halt_at & 255]
    timers.update(enumerate(setup, 0x120))
    timers.update(enumerate([0xE0F, 0xE80, 0xB00, 0xEC2, 0xB01, 0xEC2, 0xFDF], 0x102))
    audio_writes = [(120, 0xF54, 8), (121, 0xF75, 9),
                    (150, 0xF74, 7), (400, 0xF54, 0), (500, 0xF75, 7)]

    oscillator_writes = [(0, 0xF77, 3), (0, 0xF79, 2), (0, 0xF26, 1),
                         (0, 0xF78, 2), (0, 0xF7D, 15), (0, 0xF60, 10),
                         (0, 0xF7E, 15), (0, 0xF15, 1), (40, 0xF70, 8),
                         (180, 0xF70, 0), (240, 0xF70, 8)]
    return [
        ("buttons_lcd", 220, buttons, pins, []),
        ("timers_audio", 6000, timers, [], audio_writes),
        ("oscillator_gpio", 320, {0x100: 0xFF8},
         [(20, 1, 1, 0), (60, 1, 1, -1)], oscillator_writes),
    ]


def sparse(values):
    return ",".join(f"{i:X}:{v:X}" for i, v in enumerate(values) if v)


def generate():
    digest = hashlib.sha256(CPU.read_bytes() + SOUND.read_bytes()).hexdigest()
    inputs = ["# Synthetic MCU test programs only; no firmware",
              "# name|clocks(decimal)|address:opcode(hex)|step@port@mask@level|step@address@value (event steps decimal)"]
    outputs = ["# Original CPU+sound SHA256 " + digest,
               "# case|clock(decimal)|master_ticks|state|RAM|LCD|IO(127 hex nibbles)|buzzer_state",
               "# state: pc,npc,a,b,ix,iy,sp,flags,delay,halt; buzzer: shot,step,envelope_counter,on,envelope,output_level,output_frequency_index"]
    checkpoints = 0
    for name, clocks, program, pins, writes in scenarios():
        pin_text = ",".join(f"{step}@{port:X}@{mask:X}@{level}" for step, port, mask, level in pins)
        write_text = ",".join(f"{step}@{address:X}@{value:X}" for step, address, value in writes)
        inputs.append("|".join([name, str(clocks), ",".join(
            f"{address:X}:{word:X}" for address, word in sorted(program.items())), pin_text, write_text]))
        words = [0xFFB] * 0x1800
        for address, word in program.items():
            words[address] = word
        cpu, recorder = load_reference(words)
        ticks = 0
        for clock in range(clocks):
            for step, port, mask, level in pins:
                if step == clock:
                    cpu.pin_handler(("K0", "K1")[port], mask, level)
            for step, address, value in writes:
                if step == clock:
                    cpu.set_mem(address, value)
            elapsed = cpu.clock() * 64
            assert elapsed == int(elapsed)
            ticks += int(elapsed)
            if clock + 1 <= 64 or (clock + 1) % 32 == 0 or clock + 1 == clocks:
                flags = sum(int(bool(getattr(cpu, attr))) << bit
                            for bit, attr in enumerate(("_CF", "_ZF", "_DF", "_IF")))
                state = [cpu._PC, cpu._NPC, cpu._A, cpu._B, cpu._IX, cpu._IY,
                         cpu._SP, flags, int(cpu._if_delay), cpu._HALT]
                # Inspect read-to-clear registers without changing the ongoing reference run.
                irq_names = ("_IT", "_ISW", "_IPT", "_ISIO", "_IK0", "_IK1")
                saved = [getattr(cpu, attr) for attr in irq_names]
                io = "".join(f"{cpu.get_mem(0xF00 + i):X}" for i in range(0x7F))
                for attr, value in zip(irq_names, saved):
                    setattr(cpu, attr, value)
                sound = cpu._sound
                buzzer = [sound._one_shot_counter, sound._envelope_step,
                          sound._envelope_counter, int(sound._sound_on), int(sound._envelope_on),
                          recorder.level, recorder.frequency_index]
                outputs.append("|".join([name, str(clock + 1), f"{ticks:X}",
                    ",".join(f"{int(v):X}" for v in state),
                    sparse(cpu._RAM), sparse(cpu._VRAM), io,
                    ",".join(f"{int(v):X}" for v in buzzer)]))
                checkpoints += 1
    return inputs, outputs, checkpoints


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    inputs, outputs, count = generate()
    for name, lines in (("e0c6200_mcu_programs.txt", inputs), ("e0c6200_mcu_reference.txt", outputs)):
        text = "\n".join(lines) + "\n"
        path = DATA / name
        if args.check:
            assert path.read_text(encoding="utf-8") == text, f"MCU fixture differs: {path}"
        else:
            path.write_text(text, encoding="utf-8")
    print(f"{'Verified' if args.check else 'Generated'} {count} full MCU checkpoints across 6540 clocks and three integrated scenarios.")
