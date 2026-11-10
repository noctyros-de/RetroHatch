# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Opt-in, local firmware diagnostics against unchanged tamaemu sources; never distributes firmware."""
from pathlib import Path
import argparse,os,subprocess,re,array,math
ROOT=Path(__file__).resolve().parents[2]
def main():
 parser=argparse.ArgumentParser();parser.add_argument('rom',type=Path);parser.add_argument('--cycles',type=int,default=184320000);parser.add_argument("--buttons",default="");parser.add_argument("--native-log",type=Path);parser.add_argument("--native-pcm",type=Path);args=parser.parse_args()
 target=ROOT/'app/target/s1c33-boot-reference';target.mkdir(exist_ok=True,parents=True)
 vswhere=Path(os.environ.get('ProgramFiles(x86)','C:/Program Files (x86)'))/'Microsoft Visual Studio/Installer/vswhere.exe'
 vs=subprocess.check_output([str(vswhere),'-latest','-products','*','-requires','Microsoft.VisualStudio.Component.VC.Tools.x86.x64','-property','installationPath'],text=True).strip()
 sources=[ROOT/'app/tools/s1c33_boot_reference.c']+[ROOT/'references/tamaemu/src'/name for name in ['cpu.c','mem.c','lcd.c','device.c','periph.c','link.c','pn512.c','nfcpeer.c']]
 command=['cl','/nologo','/std:c11','/O2','/MT','/D_CRT_SECURE_NO_WARNINGS','/I'+str(ROOT/'references/tamaemu/src'),*map(str,sources),'/Fe:'+str(target/'reference.exe'),'/link','ws2_32.lib']
 script=target/'build.cmd';script.write_text('@echo off\ncall "'+str(Path(vs)/'Common7/Tools/VsDevCmd.bat')+'" -no_logo -arch=x64 -host_arch=x64\nif errorlevel 1 exit /b %errorlevel%\n'+subprocess.list2cmdline(command)+'\n')
 subprocess.run(['cmd','/d','/c',str(script)],cwd=target,check=True)
 with (target/'diagnostics.log').open('w')as log:
  output=subprocess.check_output([str(target/'reference.exe'),str(args.rom.resolve()),str(args.cycles),str(target/'display.ppm'),args.buttons],stderr=log,text=True)
 print(output,end='')
 if args.native_log:
  frames={};last=None
  for line in args.native_log.read_text(encoding='utf-16' if args.native_log.read_bytes().startswith((b'\xff\xfe',b'\xfe\xff'))else'utf-8-sig').splitlines():
   if line.startswith('t='):
    fields=dict(re.findall(r"(\w+)=([^ ]+)",line));last=int(float(fields['t']));frames[last]=fields
   elif line.startswith('state '):frames[last].update(re.findall(r"(\w+)=([^ ]+)",line))
  for line in output.splitlines():
   number=int(line.split()[0]);expected=dict(re.findall(r"(\w+)=([^ ]+)",line));actual=frames[number]
   for field in ['pc','cycles','ram','io','gram','irqs','lcd','tone']:
    native={'irqs':'irq','tone':'tone_events'}.get(field,field)
    assert actual[native]==expected[field],(number,field,actual[native],expected[field])
  print(f"Verified {len(output.splitlines())} actual firmware checkpoints: PC, cycles, RAM, I/O, complete GRAM, interrupts, LCD and tone-event count.")
 if args.native_pcm:
  actual=array.array('h');actual.frombytes(args.native_pcm.read_bytes());expected=array.array('h');phase=0.0;fraction=0.0;previous=0;on=False;frequency=0.0
  events=[(int(cycle),float(freq),bool(int(active)))for cycle,freq,active in (line.split()for line in (target/'display.ppm.tones').read_text().splitlines())]
  events.append((args.cycles,0.0,False))
  for cycle,new_frequency,new_on in events:
   count=(cycle-previous)*44100.0/18432000.0+fraction;n=int(count);fraction=count-n
   for _ in range(n):
    sample=0
    if on and 20<=frequency<=20000:
     phase=(phase+frequency/44100.0)%1.0;sample=4500 if phase<0.5 else -4500
    expected.append(sample)
   previous=cycle;frequency=new_frequency;on=new_on
  assert actual==expected, f"Real-firmware PCM differs from the C event oracle: {sum(a!=b for a,b in zip(actual,expected))} mismatches"
  print(f"Verified {len(actual)} real-firmware PCM samples against C tone events: exact match, no filtering.")
if __name__=='__main__':main()

