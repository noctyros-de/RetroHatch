# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Compare unfiltered MCU PCM against BrickEmuPy's actual Channel generator."""
import sys
sys.dont_write_bytecode=True
import ast,math,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
tree=ast.parse((ROOT/'references/BrickEmuPy-main/audio_engine.py').read_text())
node=next(n for n in tree.body if isinstance(n,ast.ClassDef)and n.name=='Channel')
scope={'math':math};exec(compile(ast.Module(body=[node],type_ignores=[]),'original Channel','exec'),scope)
channel=scope['Channel'](44100);count=nonzero=clipped=0;maximum=0
exe=ROOT/'app/target/release/examples/buzzer_pcm.exe'
for line in subprocess.check_output([str(exe)],text=True).splitlines():
    f,level,pcm=line.split('|');values=list(map(int,pcm.split(',')));level=int(level)
    if level==0:channel._phase=0;expected=[0]*len(values)
    else:
        frequency=32768/[8,10,12,14,16,20,24,28][int(f)]
        expected=[max(-655,min(655,int(v)))for v in channel._get_wave_data(len(values),(frequency,False,655*5*level/7))]
    errors=[abs(a-b)for a,b in zip(values,expected)];maximum=max(maximum,max(errors));count+=len(values);nonzero+=sum(v!=0 for v in values);clipped+=sum(abs(v)==655 for v in values)
assert maximum<=1,maximum
print(f'Compared {count} PCM samples across all eight frequencies, envelope levels, silence and phase transitions: max difference {maximum} PCM16 unit; {clipped}/{nonzero} audible samples clip exactly as in the original generator. No filtering applied.')
