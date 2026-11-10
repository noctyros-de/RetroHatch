# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noktyros
"""Build unchanged tamaemu C CPU/memory/LCD as an optional synthetic oracle.

No firmware is read or executed. Output and object files stay under app/target.
Use --check to verify the committed expectations; references are never modified.
"""
import argparse,hashlib,os
from pathlib import Path
import subprocess,json
ROOT=Path(__file__).resolve().parents[2]
UPSTREAM=ROOT/"references/tamaemu"
DATA=ROOT/"app/tests/data/s1c33_reference.txt"

def cases():
    pool=[0,1,0x7fffffff,0x80000000,0xffffffff,0x7fff,0x8000,0x80,31,32,0x200,0x300]
    serial=0
    def case(name,code,*,profile=0,steps=1,flags=0,r=None,**kw):
        nonlocal serial
        serial+=1
        regs=[pool[(i+serial)%len(pool)]for i in range(16)] if r is None else r
        values=[profile,steps,flags,kw.get('irq',0),kw.get('level',0),kw.get('halted',0),kw.get('hard',0),kw.get('wake',0),kw.get('clock',0),kw.get('cycles',0),kw.get('wake_at',20000),*regs,kw.get('sp',0x6000),0x87654321,0x12345678,0x4000,0x87654321,0x76543210,kw.get('pc',0x1000),len(code),*code]
        return f"{serial}_{name} "+" ".join(f"{x:x}"for x in values)
    for profile in range(8):yield case('reset',[],profile=profile,steps=0)
    pairs=[(0,1),(1,0),(2,2),(15,15)]
    for cls in range(1,6):
        for op in range(8):
            for mode in range(4):
                if cls in (2,3) and mode:continue
                for rd,rs in pairs:
                    word=(cls<<13)|(op<<10)|(mode<<8)|(rs<<4)|rd
                    if cls==5 and ((op*4+mode in (17,21))or(op*4+mode==31)):continue
                    for flags in [0,0x1f]:yield case('group', [word,0],flags=flags)
    for op in range(8):
        for imm in [0,1,31,32,63]:
            for value in pool[:9]:
                regs=[value]*16
                yield case('immediate',[0x6000|(op<<10)|(imm<<4)],flags=0x1b,r=regs)
    for prefixes in [[0xc000],[0xc001],[0xdfff],[0xc000,0xc001],[0xdfff,0xdfff]]:
        for word in [0x2801,0x2c01,0x3001,0x3801,0x3c01,0x6000,0x6c02,0x7c03,0x4001,0x5002,0xa800]:
            yield case('extended',prefixes+[word,0],flags=0x15)
    for op in range(4,16):
        for flags in range(16):
            for delayed in [0,1]:yield case('branch',[(op<<9)|(delayed<<8)|0xfc,0x6c70],flags=flags)
    for word in [0,0x40,0x80,0x200,0x240,0x2c0,0x3c0,0x480,0x4c0,0x600,0x640,0x680,0x700,0x740,0x780,0x10,0x50,0x92,0x93,0xd2,0xd3,0x1d0]:
        for rd in [0,3,15] if word not in [0x92,0x93,0xd2,0xd3] else [0]:
            yield case('control',[word|rd,0x6c41,0],flags=0x1f)
    for op in [1,2]:
        for bit in range(5):yield case('psr',[0xbf00|(op<<6)|bit],flags=0x1f)
    for word in [0,0x6c71,0x200,0x700]:
        for irq,level,flags,halted,hard,wake in [(20,3,0x10,0,0,0),(20,3,0,1,1,0),(20,1,0x210,1,1,0),(0,0,0,1,1,1),(0,0,0,1,1,0),(0,0,0,1,0,0)]:
            yield case('interrupt',[word,0],irq=irq,level=level,flags=flags,halted=halted,hard=hard,wake=wake,cycles=19992,wake_at=20000)
    for index in [0,1,2,3,8,10,11,15]:
        for flags in [0,0x1f,0xaaaaffff]:
            yield case('special_read',[0xa400|(index<<4)|6],flags=flags)
            yield case('special_write',[0xa000|(6<<4)|index],flags=flags)
    yield case('bad_fetch',[0],pc=0xffff0000)
    yield case('prefix_guard',[0xc000,0xc000,0xc000,0])
    yield case('slp_oscillator',[0x0040],clock=0x3e00)
    for word in [0x0000,0x6c71,0x1ffc]:yield case('mask28',[word,0],pc=0xf0001000)
    # Composite instruction sequences verify register/memory effects beyond a single decode.
    yield case('program',[0x6c51,0x6001,0x6401,0x3412,0x2a12,0x1e00],steps=5,r=[0x200]*16)
    for profile in range(8):
        base=0x2000000 if profile in [0,1,4,5] else 0xc00000;size=0x800000 if base==0x2000000 else 0x400000
        def operations(name,ops):return case(name,[v for op in ops for v in op],profile=profile,steps=0xffff)
        yield operations('memory_edges',[(5,0x7ffe,0x12345678),(2,0x7ffe,0),(5,0x84ffc,0xabcdef01),(2,0x84ffc,0),(1,0xffffffff,0),(3,0x900000,7),(0,0x900000,0)])
        unlock=[(3,base+0xaaa,0xaa),(3,base+0x555,0x55)]
        yield operations('flash_program',unlock+[(3,base+0xaaa,0xa0),(4,base+0x1234,0x1234)]+[(1,base+0x1234,0)]*4)
        yield operations('flash_autoselect',unlock+[(3,base+0xaaa,0x90),(1,base,0),(1,base+2,0),(2,base+0x1fe,0),(3,base,0xf0),(1,base,0)])
        for offset in [0x1234,size-0x1234]:
            ops=unlock+[(3,base+0xaaa,0xa0),(3,base+offset,0)]
            ops+=unlock+[(3,base+0xaaa,0x80)]+unlock+[(3,base+offset,0x30)]+[(1,base+offset,0)]*4
            yield operations('flash_sector',ops)
    # Register commands, RGB565/RGB666, BGR, inversion, clipping and standby.
    def command(c):return [(3,0x600000,c)]
    def data(*values):return [(3,0x600001,v)for v in values]
    initial=command(1)+command(0x11)+command(0x29)
    for colmod in [5,6]:
        for bgr in [0,8]:
            for inverted in [0x20,0x21]:
                ops=initial+command(0x3a)+data(colmod)+command(0x36)+data(bgr)
                ops+=command(0x2a)+data(0,0,0,2)+command(0x2b)+data(0,0,0,0)+command(inverted)+command(0x2c)
                ops+=data(*([0xf8,0,0x07,0xe0,0,0x1f]if colmod==5 else[0xfc,0,0,0,0xfc,0,0,0,0xfc]))
                yield case('lcd',[v for op in ops for v in op],steps=0xffff)
    yield case('lcd_sleep',[v for op in initial+command(0x10)for v in op],steps=0xffff)
    def lcd_case(name, ops):
        return case(name,[v for operation in ops for v in operation],steps=0xffff)
    # Include hidden GRAM and pending bytes, not only the visible panel hash.
    gamma = initial+command(0xe0)+data(*range(16))+command(0xe1)+data(*range(16,32))
    gamma += command(0xc0)+data(5,6)+command(0xc2)+data(7)+command(0xc5)+data(8)+command(0xc6)+data(9)+command(0)
    yield lcd_case('lcd_gamma_power', gamma)
    for xs,xe,ys,ye in [(131,133,161,163),(2,1,2,1),(127,128,127,128),(65535,65535,65535,65535)]:
        ops=initial+command(0x3a)+data(5)+command(0x2a)+data(xs>>8,xs&255,xe>>8,xe&255)+command(0x2b)+data(ys>>8,ys&255,ye>>8,ye&255)+command(0x2c)+data(0xf8,0,0x07,0xe0,0,0x1f,0xff,0xff)
        yield lcd_case('lcd_window_edge',ops)
    for colmod in [5,6]:
        for count in [1,2,3]:
            ops=initial+command(0x3a)+data(colmod)+command(0x2c)+data(*([0xf8,0x80,0x40][:count]))
            yield lcd_case('lcd_pending',ops)
        yield lcd_case('lcd_reset_retains_gram_colmod',initial+command(0x3a)+data(colmod)+command(0x2c)+data(0xf8,0x80,0x40)+command(1))
    # Real CPU byte stores program the controller and write RGB565 pixel bytes.
    program=[]
    for address,value in [(0x600000,1),(0x600000,0x11),(0x600000,0x29),(0x600000,0x3a),(0x600001,5),(0x600000,0x2c),(0x600001,0xf8),(0x600001,0)]:
        # EXT + LD.W immediate followed by ST.B r0,[r1] or [r2].
        program += [0xc000|(value>>6),0x6c00|((value&63)<<4),0x3400|((1 if address==0x600000 else 2)<<4)]
    regs=[0]*16;regs[1]=0x600000;regs[2]=0x600001
    yield case('cpu_to_lcd',program,steps=16,r=regs)


def build():
    target=ROOT/'app/target/s1c33-reference';target.mkdir(parents=True,exist_ok=True)
    sources=[ROOT/'app/tools/s1c33_reference.c']+[UPSTREAM/'src'/f for f in ['cpu.c','mem.c','lcd.c','device.c']]
    exe=target/'s1c33_reference.exe'
    if os.name!='nt':
        raise RuntimeError('The checked oracle is pinned to Windows MSVC x64; run Cargo tests with the stored fixtures on other hosts')
    vswhere=Path(os.environ.get('ProgramFiles(x86)','C:/Program Files (x86)'))/'Microsoft Visual Studio/Installer/vswhere.exe'
    vs=subprocess.check_output([str(vswhere),'-latest','-products','*','-requires','Microsoft.VisualStudio.Component.VC.Tools.x86.x64','-property','installationPath'],text=True).strip()
    devcmd=Path(vs)/'Common7/Tools/VsDevCmd.bat'
    arguments=['cl','/nologo','/std:c11','/O2','/MT','/D_CRT_SECURE_NO_WARNINGS','/I'+str(UPSTREAM/'src'),*map(str,sources),'/Fe:'+str(exe)]
    script=target/'build.cmd'
    script.write_text('@echo off\ncall "'+str(devcmd)+'" -no_logo -arch=x64 -host_arch=x64\nif errorlevel 1 exit /b %errorlevel%\n'+subprocess.list2cmdline(arguments)+'\n',encoding='utf-8')
    subprocess.run(['cmd.exe','/d','/c',str(script)],check=True,cwd=target)
    return exe

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    inputs=list(cases());env=os.environ.copy();env.pop('TAMAEMU_SLPWAKE',None)
    process=subprocess.run([str(build())],input='\n'.join(inputs)+'\n',text=True,capture_output=True,check=True,env=env)
    rows=process.stdout.splitlines();assert len(rows)==len(inputs),(len(rows),len(inputs),process.stderr[-1000:])
    hashes={p:hashlib.sha256((UPSTREAM/'src'/p).read_bytes()).hexdigest()for p in ['cpu.c','mem.c','lcd.c','device.c','emu.h']}
    text='# Synthetic tamaemu C CPU/memory/LCD oracle; no firmware\n# Windows MSVC x64 /std:c11 /O2 /MT; halfword operand order is compiler-dependent in the original C.\n# '+json.dumps(hashes,sort_keys=True)+'\n'
    for inp,out in zip(inputs,rows):assert inp.split()[0]==out.split()[0];text+=inp+' | '+' '.join(out.split()[1:])+'\n'
    if args.check:assert DATA.read_text(encoding='utf-8')==text,'S1C33 C oracle fixtures differ'
    else:DATA.parent.mkdir(parents=True,exist_ok=True);DATA.write_text(text,encoding='utf-8',newline='\n')
    print(f"{'Verified'if args.check else'Recorded'} {len(rows)} synthetic C CPU/memory/NOR/LCD cases; no MCU or firmware boot claim.")
if __name__=='__main__':main()
