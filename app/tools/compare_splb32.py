# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Direct CC0 SPLB32 oracle; synthetic fixtures are public, firmware comparisons private."""
import argparse, ast, hashlib, tempfile, subprocess
from pathlib import Path
from workspace_paths import references_dir
ROOT=Path(__file__).resolve().parents[2]
REF=references_dir()/"BrickEmuPy-main"

def load():
    ns={}
    for name in ["cores/rom.py","cores/SPLB32sound.py","interconnect.py","peripherals/HT24LC08.py","cores/SPLB32.py"]:
        module=ast.parse((REF/name).read_text());module.body=[n for n in module.body if not isinstance(n,(ast.Import,ast.ImportFrom))]
        exec(compile(module,name,"exec"),ns)
    return ns
class Host:
    def audio_handler(self,channel,data):
        if getattr(self,"capture",False):self.events.append((self.cycles,channel,0.0 if data is None else data[2]))
    def serial_tx_handler(self,*args):pass
    def svg_element_handler(self,*args):pass

def machine(ns,path):
    bus=ns["Interconnect"](Host());cpu=ns["SPLB32"]({"rom_path":str(path),"port_pullup":{"PD":6}},4_000_000,bus)
    ee=ns["HT24LC08"]({"hw_address":160,"port":"PD","SDA":2,"SCL":1,"path":None},bus)
    bus.emit_pin(None,"PB",4,0)
    return cpu,ee,bus

def fixtures(ns):
    lines=["# CC0 BrickEmuPy SPLB32 synthetic instruction oracle; opcode seed cycles pc a x y sp ps ram-sha256"]
    with tempfile.TemporaryDirectory() as tmp:
        path=Path(tmp)/"synthetic.rom"
        for opcode in range(256):
            for seed in range(4):
                data=bytearray([0xea])*524288;data[0x4800:0x4803]=bytes([opcode,0x40,0x80]);data[0x7ffc:0x7ffe]=bytes([0,0xc8]);path.write_bytes(data)
                cpu,ee,bus=machine(ns,path);handler=cpu._execute[opcode]
                if handler.__name__=="_dummy":continue
                cpu._A=[0,127,128,255][seed];cpu._X=seed;cpu._Y=3-seed;cpu._SP=[0,1,254,255][seed];cpu._set_ps([0,255,9,0x42][seed]);cpu._RAM=[(i*17+seed*63)&255 for i in range(1216)]
                cpu._PC=(cpu._PC+1)&65535;cycles=handler(cpu)
                values=[opcode,seed,cycles,cpu._PC,cpu._A,cpu._X,cpu._Y,cpu._SP,cpu._get_ps()]
                lines.append(" ".join(map(str,values))+" "+hashlib.sha256(bytes(cpu._RAM)).hexdigest())
    return "\n".join(lines)+"\n"

def firmware(ns,args):
    cpu,ee,bus=machine(ns,args.rom);cycles=0.;events={};host=bus._emulator;host.capture=bool(args.pcm);host.events=[];host.cycles=0.
    if args.events:
        for line in args.events.read_text().splitlines():
            if line.strip():t,mask=map(int,line.split());events[t]=mask
    trace=[]
    for i in range(1,args.seconds*20+1):
        if (i-1)*50 in events:
            mask=events[(i-1)*50]
            for bit in [1,2,4]:bus.emit_pin(None,"PA",bit,int(bool(mask&bit)))
        while cycles<i*200000:
            if cpu._CPU_ENBL and cpu._execute[cpu._read_mem(cpu._PC)].__name__=="_dummy":raise AssertionError(f"Reserved opcode at {cpu.pc():05x}")
            host.cycles=cycles
            cycles+=cpu.clock()
        if i%10==0:
            registers=",".join(f"{v:X}" for v in [cpu.pc(),cpu._PC,cpu._A,cpu._X,cpu._Y,cpu._SP]);io=bytes(cpu._read_mem(a)&255 for a in range(64))
            trace.append(f"{cycles:.8f}|{registers}|{bytes(cpu._RAM).hex()}|{bytes(cpu._DPRAM).hex()}|{bytes(ee._mem).hex()}|{io.hex()}")
    actual=args.trace.read_text().splitlines()
    assert len(actual)==len(trace),(len(actual),len(trace))
    for i,(a,b) in enumerate(zip(actual,trace)):
        for field,(x,y)in enumerate(zip(a.split("|"),b.split("|"))):assert x==y,f"Checkpoint {i+1}, field {field} differs: {x[:70]} != {y[:70]}"
    if args.pcm:
        import struct
        levels=[0.,0.];at=0.;pos=0.;acc=0.;pcm=[]
        for when,channel,level in host.events+[(cycles,0,0.)]:
            span=(when-at)*44100.;at=when;dac=sum(levels)*.5
            while pos+span>=4_000_000.:
                take=4_000_000.-pos;acc+=dac*take;pcm.append(int(max(-32768,min(32767,acc*8192/4_000_000.))));span-=take;pos=0.;acc=0.
            pos+=span;acc+=dac*span;levels[channel]=level
        raw=args.pcm.read_bytes();native=struct.unpack('<'+'h'*(len(raw)//2),raw);assert len(native)==len(pcm),(len(native),len(pcm));maximum=max(abs(a-b)for a,b in zip(native,pcm));assert maximum<=1,maximum
        print(f"Audio: {len(pcm)} PCM samples, {len(host.events)} reference DAC transitions, max error {maximum} PCM16 unit")
    print(f"Actual firmware: {len(trace)} checkpoints match CPU, cycles, RAM, LCD, EEPROM and SFRs")
if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--check",action="store_true");parser.add_argument("--rom",type=Path);parser.add_argument("--trace",type=Path);parser.add_argument("--events",type=Path);parser.add_argument("--pcm",type=Path);parser.add_argument("--seconds",type=int,default=5);args=parser.parse_args();ns=load()
    if args.rom:firmware(ns,args)
    else:
        target=ROOT/"app/tests/fixtures/splb32.txt";text=fixtures(ns)
        if args.check:assert target.read_text()==text,"Stale SPLB32 fixtures"
        else:
            target.parent.mkdir(parents=True,exist_ok=True);target.write_text(text,encoding="utf-8")
        print(f"SPLB32: {(len(text.splitlines())-1)//4} opcodes, {len(text.splitlines())-1} reference cases")
