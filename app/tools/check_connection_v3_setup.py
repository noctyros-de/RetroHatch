# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Opt-in real V3 setup regression; requires private firmware and a NEW private output directory.
Never modifies firmware, existing saves, references or application behavior.
"""
import argparse, hashlib, subprocess
from pathlib import Path
from workspace_paths import ROOT, target_dir

HASH="7f4da8dc89d3843b42b3ba73b9311989332706978af5367b5ef041c365a1902f"

def events(hour):
    presses=[(2000,2),(4000,2),(6000,2),(8000,hour),(10000,2),(12000,2),(14000,2),(16000,2)]
    presses += [(20000+i*4000+j*2000,mask)for i in range(5)for j,mask in enumerate([1,2])]
    presses += [(40000,2)]
    return "".join(f"{t} {mask}\n{t+100} 0\n"for t,mask in presses)

def trace(path):
    result=[]
    for line in path.read_text().splitlines():
        f=line.split("|");result.append((float(f[0])/4e6,bytes.fromhex(f[2]),bytes.fromhex(f[3]),bytes.fromhex(f[4]),bytes.fromhex(f[5])))
    return result

def clock(ram):
    r=ram[0x81-0x40:0x87-0x40]
    return r[4],r[0]+10*r[1]+60*(r[2]+10*r[3])

if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--rom",required=True,type=Path);parser.add_argument("--output",required=True,type=Path);args=parser.parse_args()
    rom=args.rom.resolve();root=args.output.resolve()
    assert not root.is_relative_to(ROOT),"Firmware-derived captures must remain outside the public repository"
    assert hashlib.sha256(rom.read_bytes()).hexdigest()==HASH,"This regression targets the verified local EU/US image only"
    root.mkdir(parents=True,exist_ok=False)
    subprocess.run(["cargo","build","--release","--locked","--example","connection_v3_probe"],cwd=ROOT/"app",check=True)
    exe=target_dir()/"release/examples/connection_v3_probe.exe"
    for name,hour,seconds in [("unset-hour",0,20),("valid-setup",1,120)]:
        path=root/(name+".events");path.write_text(events(hour),encoding="utf-8")
        run=subprocess.run([str(exe),str(rom),str(root/name),str(seconds),str(path)],capture_output=True,text=True,check=True)
        (root/(name+".log")).write_text(run.stdout+run.stderr,encoding="utf-8")
    bad=trace(root/"unset-hour/reference.trace");good=trace(root/"valid-setup/reference.trace")
    assert all(clock(ram)==(0,0)for _,ram,_,_,_ in bad),"Control unexpectedly started the game clock"
    assert all(not any(ee)for _,_,_,ee,_ in bad),"Control unexpectedly initialized EEPROM"
    assert bad[13][1]==good[13][1],"Runs differ before the isolated hour-setting input"
    # The stop guard is installed by setup, not at cold reset. Check from hour selection onward.
    assert all(ram[0xb9-0x40]&4 for _,ram,_,_,_ in bad[13:]),"Control unexpectedly cleared the firmware clock-stop flag"
    assert good[25][1][0xb9-0x40]&4==0,"Minute confirmation did not release the game-clock guard"
    assert good[33][1][0x70-0x40:0x72-0x40]==bytes([0x0a,0x22]),"Valid setup did not reach username mode"
    assert clock(good[17][1])[0]==1,"A failed to set the selected hour"
    assert clock(good[119][1])==(1,47) and clock(good[239][1])==(1,107),"Game clock did not advance exactly 60 seconds"
    assert any(good[-1][3]),"Username setup did not initialize EEPROM"
    assert good[119][2]!=good[203][2],"Expected incubation/hatch display transition is missing"
    assert hashlib.sha256(rom.read_bytes()).hexdigest()==HASH,"Original firmware changed"
    print("PASS: isolated zero-hour control, valid clock setup, username/EEPROM initialization, 60-second game clock and post-incubation display transition")
    print("Probe also verified byte-exact save continuation every five emulated seconds. Visually confirm hatch captures; this is not a universal ROM integrity certificate.")
