# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noktyros
"""Inventory the locked Windows Cargo graph and preserve cached upstream notices.

Run after fetching/building the locked Windows dependencies. No network calls,
reference changes, firmware reads or Git mutations are performed. --check refuses
stale reports. This inventory is evidence, not a legal-clearance certificate.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
TARGET = "x86_64-pc-windows-msvc"

def font_notices(path):
    data = path.read_bytes()
    tables = struct.unpack_from(">H", data, 4)[0]
    offsets = [struct.unpack_from(">4sIII", data, 12 + 16*i) for i in range(tables)]
    offset = next(row[2] for row in offsets if row[0] == b"name")
    _, count, strings = struct.unpack_from(">HHH", data, offset)
    found = set()
    for i in range(count):
        platform, _, _, name, size, start = struct.unpack_from(">HHHHHH", data, offset+6+12*i)
        if name in (0, 13, 14):
            text = data[offset+strings+start:offset+strings+start+size].decode(
                "utf-16-be" if platform in (0, 3) else "mac_roman", errors="replace")
            if text.strip():
                found.add(text.strip())
    return "\n\n".join(sorted(found))

def generate():
    raw = subprocess.check_output(["cargo", "metadata", "--locked", "--offline",
        "--filter-platform", TARGET, "--format-version", "1"], cwd=ROOT/"app")
    metadata = json.loads(raw)
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    packages = sorted((p for p in metadata["packages"] if (p["source"] or p["name"] == "tg18-core") and p["id"] in nodes),
        key=lambda p: (p["name"], p["version"]))
    apache = next(Path(p["manifest_path"]).parent/"LICENSE" for p in packages if p["name"] == "glutin").read_text(encoding="utf-8")
    egui_mit = (ROOT/"LICENSES/egui-LICENSE-MIT.txt").read_text(encoding="utf-8")
    records = []
    sections = ["RetroHatch: locked Windows Cargo dependency notices\n"
        "License texts/notices below are preserved from the cached published packages.\n"
        "Alternative licenses are retained for completeness, not cumulatively imposed.\n"
        "self_cell is used under Apache-2.0, NOT its GPL-2.0-only alternative.\n"
        "Fonts and other separately licensed data retain their own terms.\n"]
    missing = []
    for package in packages:
        base = Path(package["manifest_path"]).parent
        notices = {}
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(base).as_posix()
            name = path.name.upper()
            if name.startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT")) or (
                    package["name"] == "epaint_default_fonts" and path.suffix == ".txt"):
                notices[rel] = path.read_bytes().decode("utf-8", errors="strict")
            elif path.suffix in (".rs", ".h", ".c", ".xml"):
                text = path.read_text(encoding="utf-8", errors="replace")
                if path.suffix == ".xml":
                    for index, block in enumerate(re.findall(r"<comment>(.*?)</comment>|<!--(.*?)-->", text, re.S)):
                        block = next((b for b in block if b), "")
                        if re.search(r"copyright|licensed under|permission is hereby", block, re.I):
                            notices[f"{rel} [legal comment {index}]"] = block
                else:
                    match = re.match(r"\s*((?://[^\n]*\n|\s*\n)+|/\*.*?\*/)", text, re.S)
                    if match and re.search(r"copyright|SPDX|licensed under", match[0], re.I):
                        notices[f"{rel} [source header]"] = match[0]
            if package["name"] == "epaint_default_fonts" and path.suffix == ".ttf":
                notice = font_notices(path)
                if notice:
                    notices[f"{rel} [embedded copyright/license metadata]"] = notice
        if package["name"] in ("gl_generator", "khronos_api"):
            notices["Apache-2.0 [standard text; package declares Apache-2.0, source notices retained]"] = apache
        if package["name"] in ("eframe", "egui", "egui_glow", "egui-winit", "ecolor", "emath", "epaint", "epaint_default_fonts"):
            notices["MIT [egui release 0.37.0 workspace; standalone text omitted by some packages]"] = egui_mit
            notices["Apache-2.0 [egui release 0.37.0 workspace]"] = (ROOT/"LICENSES/egui-LICENSE-APACHE.txt").read_text(encoding="utf-8")
        if package["name"] == "profiling":
            for kind in ("MIT", "APACHE"):
                notices[f"LICENSE-{kind} [published profiling VCS revision 8271551172eb6fa4cba47369aedd93790c623df9]"] = (ROOT/f"LICENSES/profiling-LICENSE-{kind}.txt").read_text(encoding="utf-8")
        if not notices:
            missing.append(package["name"])
        record = {"name": package["name"], "version": package["version"],
            "declared_license": package["license"], "repository": package["repository"],
            "declared_authors_not_a_copyright_assignment": package["authors"],
            "features": sorted(nodes[package["id"]]["features"]),
            "notice_sections": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in sorted(notices.items())}}
        records.append(record)
        sections.append(f"\n{'='*72}\nPACKAGE {package['name']} {package['version']}\n"
            f"SOURCE {package['repository'] if package['name'] == 'tg18-core' else 'https://crates.io/crates/'+package['name']+'/'+package['version']}\n"
            f"DECLARED LICENSE {package['license']}\n")
        for name, text in sorted(notices.items()):
            sections.append(f"\n--- {name} ---\n{text}\n")
    if missing:
        raise RuntimeError(f"No notices located: {missing}")
    inventory = {"target": TARGET, "scope": "Locked default Windows normal/build dependencies; not other targets or toolchains",
        "cargo_lock_sha256": hashlib.sha256((ROOT/"app/Cargo.lock").read_text(encoding="utf-8").encode("utf-8")).hexdigest(), "packages": records}
    outputs = {ROOT/"LICENSES/CARGO_PACKAGES.json": json.dumps(inventory, indent=2, ensure_ascii=False)+"\n",
        ROOT/"LICENSES/CARGO_NOTICES.txt": "".join(sections)}
    table = ["# Locked Windows dependency inventory\n",
        "Generated by `python -B app/tools/audit_licenses.py`. Copyright notices are in",
        "[CARGO_NOTICES.txt](CARGO_NOTICES.txt); declared package authors are metadata, not an ownership claim.",
        "The source/build graph is a conservative superset of code included in the executable.",
        "See [THIRD_PARTY_LICENSES.md](../THIRD_PARTY_LICENSES.md) for mandatory compound licenses and limitations.\n",
        "| Package | Version | Declared license | Upstream |", "| --- | --- | --- | --- |"]
    for p in records:
        table.append(f"| {p['name']} | {p['version']} | `{p['declared_license']}` | [source]({p['repository'] if p['name'] == 'tg18-core' else 'https://crates.io/crates/'+p['name']+'/'+p['version']}) |")
    outputs[ROOT/"LICENSES/CARGO_INVENTORY.md"] = "\n".join(table)+"\n"
    return outputs, len(records)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs, count = generate()
    for path, text in outputs.items():
        encoded = text.encode("utf-8")
        if args.check:
            assert path.read_bytes().replace(b"\r\n", b"\n") == encoded.replace(b"\r\n", b"\n"), f"Stale dependency evidence: {path.name}"
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(encoded)
    print(f"{'Verified' if args.check else 'Recorded'} {count} locked Windows packages and their available full license/legal notices; release review remains required.")
