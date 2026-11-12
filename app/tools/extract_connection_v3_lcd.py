# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Extract factual V3 wiring; optional exact symbol coverage stays private, never bundled.
Requires local BrickEmuPy and PyQt6. This tool does not establish artwork rights.
"""
import argparse,math,re,struct,subprocess,xml.etree.ElementTree as ET
from pathlib import Path
from workspace_paths import references_dir
ROOT=Path(__file__).resolve().parents[2]
SOURCE=references_dir()/"BrickEmuPy-main/assets/Tamagotchi_v3.svg"
TARGET=ROOT/"app/src/core/splb32/lcd.rs"
def extract():
    from PyQt6.QtSvg import QSvgRenderer
    renderer=QSvgRenderer(str(SOURCE));items=[]
    for element in ET.parse(SOURCE).getroot().iter():
        key=element.get("id","");match=re.fullmatch(r"(\d+)_(\d)",key)
        if match:
            ram,bit=map(int,match.groups());bounds=renderer.boundsOnElement(key);items.append((ram,bit,bounds,key))
    assert len(items)==970
    left=min(b.x()for _,_,b,_ in items);top=min(b.y()for _,_,b,_ in items);right=max(b.right()for _,_,b,_ in items);scale=352/(right-left)
    matrix=[s for s in items if s[0]>=16];assert len(matrix)==960
    rows={}
    for ram,bit,b,key in matrix:rows.setdefault(round(b.y(),1),[]).append((b.x(),ram*8+bit))
    assert len(rows)==30 and all(len(row)==32 for row in rows.values())
    text="// SPDX-License-Identifier: GPL-3.0-only\n// Technical electrical mapping from BrickEmuPy Tamagotchi_v3.svg (CC0 declaration).\n// No original icon paths or firmware artwork.\npub const MATRIX: [[u16;32];30]=[\n"
    for _,row in sorted(rows.items()):text+="["+",".join(str(bit)for _,bit in sorted(row))+"],\n"
    text+="];\npub const RECTS:&[(u16,u16,u16,u16,u16)]=&[\n"
    for ram,bit,b,key in matrix:text+=str((ram*8+bit,round((b.x()-left)*scale),round((b.y()-top)*scale),round(b.width()*scale),round(b.height()*scale)))+",\n"
    text+="];\n"
    text=subprocess.run(["rustfmt","--edition","2024"],input=text,text=True,capture_output=True,check=True).stdout
    return text,renderer,items,left,top,scale
if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--check",action="store_true");parser.add_argument("--private-output",type=Path);args=parser.parse_args();text,renderer,items,left,top,scale=extract()
    if args.check:assert TARGET.read_text()==text,"Stale V3 factual LCD mapping"
    else:TARGET.write_text(text,encoding="utf-8")
    if args.private_output:
        path=args.private_output.resolve();assert not path.is_relative_to(ROOT),"Exact icon assets must stay outside the public repository"
        from PyQt6.QtGui import QImage,QPainter
        from PyQt6.QtCore import QRectF,Qt
        data=bytearray(b"RHV3LCD1"+struct.pack("<HHH",352,363,len(items)))
        for ram,bit,b,key in items:
            if ram>=16:
                x,y,w,h=round((b.x()-left)*scale),round((b.y()-top)*scale),round(b.width()*scale),round(b.height()*scale);alpha=bytes([255])*w*h
            else:
                x,y=math.floor((b.x()-left)*scale),math.floor((b.y()-top)*scale);w=math.ceil((b.right()-left)*scale)-x;h=math.ceil((b.bottom()-top)*scale)-y
                im=QImage(w*4,h*4,QImage.Format.Format_ARGB32);im.fill(0);painter=QPainter(im);painter.setRenderHint(QPainter.RenderHint.Antialiasing);renderer.render(painter,key,QRectF(((b.x()-left)*scale-x)*4,((b.y()-top)*scale-y)*4,b.width()*scale*4,b.height()*scale*4));painter.end();im=im.scaled(w,h,Qt.AspectRatioMode.IgnoreAspectRatio,Qt.TransformationMode.SmoothTransformation);alpha=bytes(im.pixelColor(xx,yy).alpha()for yy in range(h)for xx in range(w))
            assert x>=0 and y>=0 and x+w<=352 and y+h<=363
            data+=struct.pack("<HBHHHH",ram,bit,x,y,w,h)+alpha
        path.parent.mkdir(parents=True,exist_ok=True)
        with path.open("xb")as output:output.write(data)
        print("Private exact LCD coverage written; publication rights remain unresolved")
    print("Verified 960 matrix bits and 10 fixed-symbol addresses; no shell or ROM artwork copied")
