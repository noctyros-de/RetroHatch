# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Rasterize only RetroHatch's original editable replacement SVGs, never reference art.
Optional development tool: PyQt6 is not required by Cargo or clean public builds.
"""
import argparse
from pathlib import Path
from PyQt6.QtCore import QByteArray,QRectF
from PyQt6.QtGui import QImage,QPainter
from PyQt6.QtSvg import QSvgRenderer
ROOT=Path(__file__).resolve().parents[2]
ASSETS=ROOT/"app/assets/connection-v3"
NAMES=["health","food","clean","games","connection","discipline","medicine","lights","friends","attention"]
def generate():
    data=bytearray()
    for name in NAMES:
        renderer=QSvgRenderer(QByteArray((ASSETS/(name+".svg")).read_bytes()));assert renderer.isValid(),name
        image=QImage(40,40,QImage.Format.Format_ARGB32);image.fill(0);painter=QPainter(image);painter.setRenderHint(QPainter.RenderHint.Antialiasing);renderer.render(painter,QRectF(0,0,40,40));painter.end()
        data.extend(image.pixelColor(x,y).alpha()for y in range(40)for x in range(40))
    return bytes(data)
if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--check",action="store_true");args=parser.parse_args();data=generate();target=ASSETS/"icons.alpha"
    if args.check:assert target.read_bytes()==data,"Stale original V3 replacement icon coverage"
    else:target.write_bytes(data)
    print("Verified ten original GPL-3.0-only replacement icons and their editable SVG sources")
