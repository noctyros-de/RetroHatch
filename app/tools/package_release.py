# SPDX-License-Identifier: GPL-3.0-only
# SPDX-FileCopyrightText: 2026 Noctyros
"""Create local Windows/source release ZIPs from an allowlisted source tree.

Never uploads, changes Git state, or reads firmware/user-data directories.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import tempfile
import tomllib
import zipfile

from workspace_paths import target_dir, release_dir

ROOT = Path(__file__).resolve().parents[2]
ROOT_FILES = {".vscode/settings.json", ".vscode/tasks.json", ".vscode/extensions.json", "AGENTS.md", ".gitignore", "LICENSE", "README.md", "COPYRIGHT.md",
              "THIRD_PARTY_LICENSES.md", "CONTRIBUTING.md", "SECURITY.md", "docs/assets/retrohatchlogo.png"}
APP_FILES = {"app/build.rs", "app/.gitignore", "app/Cargo.toml", "app/Cargo.lock", "app/THIRD_PARTY_NOTICES.md"}
PREFIXES = (".github/", "developer/", "LICENSES/", "app/src/", "app/assets/lcd/",
            "app/examples/", "app/tests/", "app/tools/", "app/LICENSES/", "app/third_party/")
SUFFIXES = {".rs", ".toml", ".lock", ".md", ".txt", ".py", ".ps1", ".json",
            ".html", ".alpha", ".yml", ".yaml", ".c"}
V3_ASSETS = {"app/assets/connection-v3/"+name for name in (
    "README.md", "icons.alpha", "health.svg", "food.svg", "clean.svg", "games.svg",
    "connection.svg", "discipline.svg", "medicine.svg", "lights.svg", "friends.svg", "attention.svg")}
GALLERY = PurePosixPath("docs/screenshots")
GALLERY_IMAGES = {".png", ".jpg", ".jpeg", ".webp", ".gif"}
SYSTEM_DLLS = {"kernel32.dll", "kernelbase.dll", "user32.dll", "dwmapi.dll",
              "ole32.dll", "combase.dll", "oleaut32.dll", "shell32.dll", "shlwapi.dll", "uxtheme.dll",
              "winmm.dll", "comdlg32.dll", "comctl32.dll", "advapi32.dll", "bcrypt.dll",
              "bcryptprimitives.dll", "opengl32.dll", "gdi32.dll", "imm32.dll",
              "ntdll.dll", "ws2_32.dll", "secur32.dll", "userenv.dll", "version.dll",
              "setupapi.dll", "msvcrt.dll", "ucrtbase.dll"}


def source_allowed(name):
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or "\\" in name or ":" in name:
        return False
    if path.parent == GALLERY and (path.name == "README.md" or path.suffix.lower() in GALLERY_IMAGES):
        return True
    if name in ROOT_FILES or name in APP_FILES or name in V3_ASSETS:
        return True
    return name.startswith(PREFIXES) and (path.suffix.lower() in SUFFIXES or path.name == "LICENSE")


def release_names(version):
    """Derive archive names from the sole application version in Cargo.toml."""
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version):
        raise ValueError("Invalid Cargo semantic version")
    return f"RetroHatch-v{version}-Windows.zip", f"RetroHatch-v{version}-source.zip"


def pe_imports(data):
    """Read normal and delay-load imports; reject unexpected PE layouts."""
    if data[:2] != b"MZ":
        raise ValueError("Not a Windows executable")
    pe = struct.unpack_from("<I", data, 60)[0]
    if data[pe:pe+4] != b"PE\0\0":
        raise ValueError("Invalid PE signature")
    if struct.unpack_from("<H", data, pe+4)[0] != 0x8664:
        raise ValueError("The release executable must target Windows x86_64")
    count = struct.unpack_from("<H", data, pe+6)[0]
    opt = pe+24
    size = struct.unpack_from("<H", data, pe+20)[0]
    if struct.unpack_from("<H", data, opt)[0] != 0x20b:
        raise ValueError("Expected PE32+ executable")
    sections = [struct.unpack_from("<8sIIII", data, opt+size+40*i) for i in range(count)]

    def offset(rva):
        for _, virtual_size, address, raw_size, raw in sections:
            if address <= rva < address+max(virtual_size, raw_size):
                return raw+rva-address
        raise ValueError("Invalid PE address")

    def name_at(rva):
        start = offset(rva)
        return data[start:data.index(0, start)].decode("ascii").lower()

    imports = set()
    for index, stride, field in ((1, 20, 12), (13, 32, 4)):
        rva = struct.unpack_from("<I", data, opt+112+8*index)[0]
        if rva:
            cursor = offset(rva)
            while any(data[cursor:cursor+stride]):
                if index == 13 and not struct.unpack_from("<I", data, cursor)[0] & 1:
                    raise ValueError("Unsupported absolute delay-import descriptor")
                imports.add(name_at(struct.unpack_from("<I", data, cursor+field)[0]))
                cursor += stride
    if not imports:
        raise ValueError("Executable has no recognizable DLL imports")
    return sorted(imports)


def validate_executable(data, private_roots):
    imports = pe_imports(data)
    unexpected = [name for name in imports if name not in SYSTEM_DLLS
                  and not name.startswith(("api-ms-win-", "ext-ms-win-"))]
    if unexpected:
        raise ValueError(f"Non-system DLLs require redistribution review: {unexpected}")
    folded = data.lower()
    for root in private_roots:
        if root:
            for form in (str(root), str(root).replace("\\", "/")):
                if any(form.lower().encode(encoding) in folded for encoding in ("utf-8", "utf-16-le")):
                    raise ValueError("Executable contains a private build-path prefix; rebuild with build_release.ps1")
    return imports


def write_zip(path, folder):
    # Exclusive creation prevents accidentally replacing a previous candidate.
    with zipfile.ZipFile(path, "x", zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=False) as archive:
        for file in sorted(folder.rglob("*")):
            if file.is_symlink():
                raise ValueError("Symlinks are not allowed in release packages")
            if file.is_file():
                archive.write(file, file.relative_to(folder).as_posix())
    with zipfile.ZipFile(path) as archive:
        if archive.testzip():
            raise ValueError("ZIP integrity check failed")


def run(*args, cwd=ROOT):
    return subprocess.check_output(args, cwd=cwd, text=True, encoding="utf-8").strip()


def package(args):
    metadata = tomllib.loads((ROOT/"app/Cargo.toml").read_text(encoding="utf-8"))
    version = metadata["package"]["version"]
    binary_name, source_name = release_names(version)
    if args.tag != "v"+version:
        raise ValueError("Tag must match the Cargo.toml package version")
    dirty = bool(run("git", "status", "--porcelain"))
    if dirty and not args.allow_dirty:
        raise ValueError("Source tree is not clean; --allow-dirty is for local draft verification only")
    revision = run("git", "rev-parse", "HEAD")
    toolchain = run("rustc", "-Vv")
    expected = (ROOT/"LICENSES/rust-stdlib/TOOLCHAIN.txt").read_text(encoding="utf-8").splitlines()[0]
    if toolchain.splitlines()[0] != expected:
        raise ValueError("Toolchain notices must be re-audited for this Rust version")
    executable = target_dir()/"release/RetroHatch.exe"
    imports = validate_executable(executable.read_bytes(), [os.environ.get("USERPROFILE"), ROOT,
                                                          run("rustc", "--print", "sysroot")])
    files = run("git", "ls-files", "-z", "--cached", "--others", "--exclude-standard").split("\0")
    files = sorted(set(filter(None, files)))
    rejected = [name for name in files if not source_allowed(name)]
    if rejected:
        raise ValueError(f"Files outside the publication allowlist: {rejected}")
    required = {"app/src/main.rs", "app/Cargo.toml", "app/Cargo.lock", "LICENSE", "README.md",
                "COPYRIGHT.md", "THIRD_PARTY_LICENSES.md", "LICENSES/CARGO_PACKAGES.json",
                "app/LICENSES/BrickEmuPy-CC0-1.0.txt", ".github/workflows/windows-release.yml"}
    required |= V3_ASSETS
    if not required.issubset(files):
        raise ValueError(f"Missing required publication files: {sorted(required-set(files))}")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any((output/name).exists() for name in (binary_name, source_name, "SHA256SUMS.txt", "RELEASE_NOTES.md")):
        raise ValueError("Output already contains release artifacts; select a new output directory")
    with tempfile.TemporaryDirectory(prefix="retrohatch-package-") as temporary:
        stage = Path(temporary)
        source = stage/"source"
        source.mkdir()
        source_hashes = {}
        for name in files:
            original = ROOT/name
            if original.is_symlink() or not original.is_file():
                raise ValueError(f"Publication source is not a regular file: {name}")
            destination = source/name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, destination)
            source_hashes[name] = hashlib.sha256(destination.read_bytes()).hexdigest()
        vendor_args = ["cargo", "vendor", "--locked", "--versioned-dirs", str(source/"vendor")]
        if args.offline:
            vendor_args.append("--offline")
        configuration = run(*vendor_args, cwd=source/"app")
        config = tomllib.loads(configuration)
        if set(config.get("source", {})) != {"crates-io", "vendored-sources"}:
            raise ValueError("Unexpected dependency source; review vendoring configuration")
        (source/".cargo").mkdir()
        (source/".cargo/config.toml").write_text(
            '[source.crates-io]\nreplace-with = "vendored-sources"\n\n'
            '[source.vendored-sources]\ndirectory = "vendor"\n', encoding="utf-8")
        info = {"version": version, "tag": args.tag, "revision": revision,
                "local_draft": dirty, "target": "x86_64-pc-windows-msvc", "toolchain": toolchain,
                "command": "cargo build --release --locked", "flags_script": "app/tools/build_release.ps1",
                "dll_imports": imports, "source_sha256": source_hashes,
                "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest()}
        manifest = json.dumps(info, indent=2)+"\n"
        source_info = f"""# Matching source and build information

Version: {args.tag}
Revision: {revision}
Local uncommitted draft: {dirty}
Matching source archive: {source_name}
Repository: https://github.com/noctyros-de/RetroHatch

The source ZIP is supplied beside the Windows ZIP in the same GitHub Release.
It includes locked vendored Cargo dependencies and an offline configuration.
Install Rust 1.99.1 (x86_64-pc-windows-msvc) and Visual Studio C++ Build Tools
with a Windows SDK to rebuild. From app/, run cargo build --release --locked
--offline. To reproduce the distribution flags, run the source root's
app/tools/build_release.ps1 wrapper. BUILD_INFO.json records the exact source
file hashes, toolchain and EXE hash; reproducible byte-for-byte output is not
guaranteed. Read developer/RELEASE.md for redistribution and provenance gates.

The Windows ZIP requires no Rust, Visual Studio or separate MSVC runtime
installation. It needs 64-bit Windows 10/11 and an OpenGL 3.3-capable driver.
ROMs, firmware, personal libraries, settings and saves are not included.
"""
        for directory in (source, stage/"windows"):
            directory.mkdir(exist_ok=True)
            (directory/"SOURCE.md").write_text(source_info, encoding="utf-8")
            (directory/"BUILD_INFO.json").write_text(manifest, encoding="utf-8")
        binary = stage/"windows"
        shutil.copyfile(executable, binary/"RetroHatch.exe")
        for name in files:
            if name.startswith("docs/screenshots/") or name in ROOT_FILES-{".gitignore", "AGENTS.md", ".vscode/settings.json", ".vscode/tasks.json", ".vscode/extensions.json"} or name.startswith(("LICENSES/", "developer/", "app/LICENSES/")) or name in {"app/THIRD_PARTY_NOTICES.md", "app/third_party/tg18-core/LICENSE"}:
                destination = binary/name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source/name, destination)
        write_zip(output/binary_name, binary)
        write_zip(output/source_name, source)
    hashes = "".join(f"{hashlib.sha256((output/name).read_bytes()).hexdigest()}  {name}\n"
                     for name in (binary_name, source_name))
    (output/"SHA256SUMS.txt").write_text(hashes, encoding="utf-8")
    (output/"RELEASE_NOTES.md").write_text(
        f"# RetroHatch {args.tag}\n\n"
        f"Windows: download **{binary_name}**, extract it, and run **RetroHatch.exe**. "
        "No Rust or Visual Studio installation is required. 64-bit Windows 10/11 "
        "and an OpenGL 3.3-capable graphics driver are required.\n\n"
        f"Matching source and vendored dependencies: **{source_name}**. "
        f"Source revision: `{revision}`. Verify downloads against **SHA256SUMS.txt**.\n\n"
        "No copyrighted ROMs or firmware are included. Import your own appropriately "
        "licensed firmware. RetroHatch is an unofficial fan project, unaffiliated "
        "with Bandai. See the included GPL and third-party notices.\n", encoding="utf-8")
    print(f"Verified Windows imports: {', '.join(imports)}")
    print(f"Created {binary_name}, {source_name} and SHA256SUMS.txt; no upload performed.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--output", type=Path, default=release_dir())
    parser.add_argument("--allow-dirty", action="store_true")
    parser.add_argument("--offline", action="store_true")
    package(parser.parse_args())
