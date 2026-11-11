# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Compare local Yasashii firmware against unchanged BrickEmuPy; no firmware is distributed."""
import sys
sys.dont_write_bytecode=True
import argparse, hashlib, json
from fractions import Fraction
from workspace_paths import references_dir
from pathlib import Path
from compare_e0c6200_mcu import load_reference
from compare_genjintch import state, MASTER_HZ
PROFILE=json.loads((references_dir()/"BrickEmuPy-main/assets/TamagotchiYasashii.brick").read_text(encoding="utf-8"))
assert PROFILE['core']=='E0C6200' and PROFILE['clock']==1060000 and PROFILE['RAM']==768
assert PROFILE['mask_options']['port_pushpull']=={f'R{i}':15 for i in range(5)}
assert {(v['port'],v['mask'],v['level']) for v in PROFILE['peripherals']['direct_input'].values() if v['port']!='RES'} == {(port,mask,0) for port in ('K0','K1') for mask in (1,2,4,8)}
def compare_rom(rom, trace, *, clock=1060000, rom_bytes=16384):
    data=rom.read_bytes()
    assert len(data)==rom_bytes and all(data[i] < 16 for i in range(0,len(data),2)), f"Expected a {rom_bytes}-byte unpacked E0C6200 image"
    words=[int.from_bytes(data[i:i+2],"big") for i in range(0,len(data),2)]
    cpu, _=load_reference(words,clock,mask=PROFILE["mask_options"])
    elapsed=Fraction(); clocks=0; checkpoints=0; events=0; displays=set()
    for line in trace.read_text(encoding="utf-8").splitlines():
        fields=line.split('|')
        if fields[0] not in ('S','E'): continue
        target=int(fields[1])
        while clocks<target:
            elapsed += Fraction(cpu.clock()); clocks+=1
        assert clocks==target
        if fields[0]=='E':
            cpu.pin_handler(('K0','K1')[int(fields[4])],int(fields[2]),0 if fields[3]=='1' else -1)
            events+=1; continue
        context=f"clock {clocks}"
        assert int(fields[2])==int(elapsed*MASTER_HZ/clock),(context,'time')
        assert [int(v,16) for v in fields[3].split(',')]==state(cpu),(context,'CPU')
        assert fields[4]==''.join(f'{v:X}' for v in cpu._RAM),(context,'RAM')
        assert fields[5]==''.join(f'{v:X}' for v in cpu._VRAM),(context,'LCD')
        assert fields[6]==f'{cpu.get_mem(0xF70):X},{cpu.get_mem(0xF71):X}',(context,'oscillator/display control')
        vram=cpu.get_VRAM()
        expected_leds=''.join(str(int(bool(v & 1))) for v in vram[160:163]+vram[164:167]) if len(vram)>160 else '000000'
        assert fields[7] == expected_leds, (context,'GPIO indicators')
        displays.add(fields[5]); checkpoints+=1
    print(f"Verified user ROM SHA256 {hashlib.sha256(data).hexdigest()}: {checkpoints} complete CPU/RAM/LCD checkpoints, {events} input transitions, {clocks} original clocks, {len(displays)} distinct LCD states, {float(elapsed/clock):.6f} seconds.")

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--rom',type=Path,required=True);p.add_argument('--trace',type=Path,required=True)
    a=p.parse_args();compare_rom(a.rom,a.trace)
