# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Compare production alpha coverage with exact Qt SVG at identical physical scales.

Requires the release lcd_probe example and development-only PyQt6. Writes only
temporary PGM images. Matrix pixels are deliberately excluded from antialiasing.
"""
import sys
sys.dont_write_bytecode=True
import argparse,re,subprocess,tempfile,xml.etree.ElementTree as ET
from pathlib import Path
from workspace_paths import references_dir, target_dir
REFERENCES = references_dir()
from PyQt6.QtCore import QByteArray,QRectF
from PyQt6.QtGui import QImage,QPainter
from PyQt6.QtSvg import QSvgRenderer
from extract_lcd_masks import ROOT,PROFILES,icon_source

def reference(node,left,top,scale,box):
    x,y,w,h=box
    raw=icon_source(node,(left+x/scale,top+y/scale,w/scale,h/scale))
    renderer=QSvgRenderer(QByteArray(raw));assert renderer.isValid()
    img=QImage(w,h,QImage.Format.Format_ARGB32_Premultiplied);img.fill(0)
    painter=QPainter(img);renderer.render(painter,QRectF(0,0,w,h));painter.end()
    img=img.convertToFormat(QImage.Format.Format_Alpha8);b=img.constBits().asstring(img.sizeInBytes());stride=img.bytesPerLine()
    return [v for row in range(h) for v in b[row*stride:row*stride+w]]

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--probe',type=Path,default=target_dir()/'release/examples/lcd_probe.exe');args=p.parse_args()
    for key,name,left,top in PROFILES:
        root=ET.fromstring((REFERENCES/f'BrickEmuPy-main/assets/{name}.svg').read_bytes());display=next(n for n in root.iter()if n.get('id')=='display');nodes=[n for n in display if not n.get('id','').startswith('col_')]
        oldname={'p1':'lcd_geometry','genjintch':'genjintch_lcd_geometry','angel':'angel_lcd_geometry','morino':'morino_lcd_geometry','umino':'umino_lcd_geometry'}[key]
        text=(ROOT/f'app/src/{oldname}.rs').read_text().split('pub const ICON_SPANS')[1]
        spans=[[tuple(map(int,t)) for t in re.findall(r'\((\d+),\s*(\d+),\s*(\d+)\)',part)] for part in re.findall(r'&\[(.*?)\]',text,re.S) if re.search(r'\(\d+,',part)]
        assert len(spans)==8,(key,len(spans))
        for scale in (.65,.85,1,1.25,1.5,2,2.5):
            error=legacy_error=area=0;max_error=0
            with tempfile.TemporaryDirectory(prefix='retrohatch-lcd-')as folder:
                lines=subprocess.check_output([str(args.probe),key,str(scale),folder],text=True).splitlines()
                for line,node,old in zip(lines,nodes,spans):
                    i,x,y,w,h=map(int,line.split());data=(Path(folder)/f'{i}.pgm').read_bytes().split(b'\n',3)[3];assert len(data)==w*h
                    ref=reference(node,left,top,scale,(x,y,w,h));errs=[abs(a-b)for a,b in zip(data,ref)];error+=sum(errs);max_error=max(max_error,max(errs));area+=len(ref)
                    # Old integer scanlines as geometric area coverage, avoiding a bias from renderer seams.
                    legacy=[0.]*(w*h)
                    for sx,sy,length in old:
                        a,b,c,d=sx*scale-x,sy*scale-y,(sx+length)*scale-x,(sy+1)*scale-y
                        for yy in range(max(0,int(b)),min(h,int(d)+1)):
                            for xx in range(max(0,int(a)),min(w,int(c)+1)):
                                legacy[yy*w+xx]+=255*max(0,min(c,xx+1)-max(a,xx))*max(0,min(d,yy+1)-max(b,yy))
                    legacy_error+=sum(abs(min(255,a)-b)for a,b in zip(legacy,ref))
            print(f'{key:9} scale={scale:4.2f} Qt alpha MAE={error/area:.3f}/255 (old={legacy_error/area:.3f}), max={max_error}')
            assert error/area < 3.0,(key,scale,error/area)
            assert error < legacy_error,(key,scale,'no measured improvement')
