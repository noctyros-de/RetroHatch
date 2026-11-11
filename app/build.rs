// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Cargo-derived Windows version information with optional private ICO branding.
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
fn main() {
    println!("cargo:rerun-if-env-changed=RETROHATCH_UNBRANDED");
    println!("cargo:rerun-if-env-changed=RETROHATCH_RESOURCES_DIR");
    println!("cargo:rerun-if-env-changed=RETROHATCH_LOCAL_DIR");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    println!("cargo:rerun-if-changed=Cargo.toml");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let resources = env::var_os("RETROHATCH_RESOURCES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("RETROHATCH_LOCAL_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("../../local"))
                .join("resources")
        });
    let icon = resources.join("retrohatchicon.ico");
    // Watch the directory too: adding/removing an optional icon must invalidate the build.
    println!("cargo:rerun-if-changed={}", resources.display());
    println!("cargo:rerun-if-changed={}", icon.display());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut png = Vec::new();
    let enabled = env::var("RETROHATCH_UNBRANDED").as_deref() != Ok("1");
    if enabled && icon.is_file() {
        let bytes = fs::read(&icon).expect("Cannot read local application icon");
        // The supplied official ICO is PNG-backed. Preserve the original pixels and ICO bytes.
        if bytes.len() < 6 || bytes[..4] != [0, 0, 1, 0] {
            panic!("Invalid local ICO header")
        }
        let count = u16::from_le_bytes(bytes[4..6].try_into().unwrap()) as usize;
        let mut best = 0;
        for n in 0..count {
            let at = 6 + n * 16;
            let entry = bytes.get(at..at + 16).expect("Truncated ICO directory");
            let size = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            let data = bytes
                .get(offset..offset.checked_add(size).expect("Invalid ICO offset"))
                .expect("Truncated ICO image");
            let width = if entry[0] == 0 {
                256
            } else {
                entry[0] as usize
            };
            if data.starts_with(b"\x89PNG\r\n\x1a\n") && width > best {
                png = data.to_vec();
                best = width;
            }
        }
        if png.is_empty() {
            println!(
                "cargo:warning=The local ICO has no PNG frame; viewport uses the standard fallback icon"
            );
        }
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let branded = enabled && icon.is_file();
        if branded {
            fs::copy(&icon, out.join("retrohatch.ico")).expect("Cannot stage the local icon");
        }
        let version = env::var("CARGO_PKG_VERSION").expect("Cargo package version is missing");
        fs::write(
            out.join("application.rc"),
            version_resource(&version, branded),
        )
        .unwrap();
        let rc = find_rc().expect("Windows version metadata requires the Windows SDK resource compiler rc.exe (or set RC to its path)");
        let output = Command::new(rc)
            .current_dir(&out)
            .arg("/nologo")
            .arg("/fo")
            .arg(out.join("application.res"))
            .arg(out.join("application.rc"))
            .output()
            .expect("Cannot launch Windows resource compiler");
        assert!(
            output.status.success(),
            "Windows application resource compilation failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!(
            "cargo:rustc-link-arg-bin=RetroHatch={}",
            out.join("application.res").display()
        );
    }
    fs::write(out.join("icon.png"), png).unwrap();
}

/// Windows fixed fields use four 16-bit numbers; strings retain the full Cargo SemVer.
fn version_resource(version: &str, branded: bool) -> String {
    let base = version.split(['-', '+']).next().unwrap();
    let numbers: Vec<u16> = base
        .split('.')
        .map(|n| {
            n.parse()
                .expect("Windows version components must fit unsigned 16-bit fields")
        })
        .collect();
    assert_eq!(numbers.len(), 3, "Expected MAJOR.MINOR.PATCH Cargo version");
    assert!(
        version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b)),
        "Invalid Cargo version"
    );
    let numeric = format!("{},{},{},0", numbers[0], numbers[1], numbers[2]);
    let icon = if branded {
        "101 ICON \"retrohatch.ico\"\n"
    } else {
        ""
    };
    format!(
        r#"#pragma code_page(65001)
{icon}1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "Finn Lülsdorf / Noctyros\0"
            VALUE "FileDescription", "RetroHatch Tamagotchi Emulator\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "InternalName", "RetroHatch\0"
            VALUE "LegalCopyright", "Copyright (C) 2026 Finn Lülsdorf / Noctyros; original RetroHatch contributions.\0"
            VALUE "OriginalFilename", "RetroHatch.exe\0"
            VALUE "ProductName", "RetroHatch\0"
            VALUE "ProductVersion", "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cargo_version_and_attribution_are_embedded_with_or_without_branding() {
        for version in ["0.1.0", "0.3.2", "1.0.0-beta.1+local"] {
            for branded in [false, true] {
                let rc = version_resource(version, branded);
                assert_eq!(rc.contains("101 ICON"), branded);
                assert!(rc.contains(&format!("VALUE \"FileVersion\", \"{version}\\0\"")));
                assert!(rc.contains(&format!("VALUE \"ProductVersion\", \"{version}\\0\"")));
                assert!(rc.contains("Finn Lülsdorf / Noctyros"));
                assert!(rc.contains("VALUE \"OriginalFilename\", \"RetroHatch.exe\\0\""));
            }
        }
        assert!(version_resource("0.3.2", false).contains("FILEVERSION 0,3,2,0"));
        assert!(version_resource("1.0.0-beta.1+local", false).contains("PRODUCTVERSION 1,0,0,0"));
    }
    #[test]
    fn invalid_or_unrepresentable_windows_versions_are_rejected() {
        for version in ["0.1", "0.1.0.0", "65536.0.0", "0.1.0\"bad"] {
            assert!(std::panic::catch_unwind(|| version_resource(version, false)).is_err());
        }
    }
}

fn find_rc() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RC") {
        return Some(path.into());
    }
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }
    let base = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)?
        .join("Windows Kits/10/bin");
    let mut versions: Vec<_> = fs::read_dir(base)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|p| p.join("x64/rc.exe"))
        .find(|p| Path::is_file(p))
}
