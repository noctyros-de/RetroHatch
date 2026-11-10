// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Native ROM selection; no firmware downloading or automatic file execution.
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub fn pick_rom(folder: &Path) -> Result<Option<PathBuf>, String> {
    pick_file(folder, false, false)
}

#[cfg(windows)]
fn pick_file(folder: &Path, save: bool, state: bool) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let filter: Vec<u16> = (if state {
        "RetroHatch Save (*.rhstate)\0*.rhstate\0All files\0*.*\0\0"
    } else {
        "Tamagotchi ROM (*.bin;*.rom)\0*.bin;*.rom\0All files\0*.*\0\0"
    })
    .encode_utf16()
    .collect();
    let title: Vec<u16> = (if save {
        "Export RetroHatch Save"
    } else if state {
        "Import RetroHatch Save"
    } else {
        "Import user-provided Tamagotchi ROM"
    })
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let initial: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut path = vec![0u16; 32768];
    let extension: Vec<u16> = "rhstate".encode_utf16().chain(Some(0)).collect();
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFile: path.as_mut_ptr(),
        nMaxFile: path.len() as u32,
        lpstrFilter: filter.as_ptr(),
        lpstrTitle: title.as_ptr(),
        lpstrInitialDir: initial.as_ptr(),
        lpstrDefExt: if state {
            extension.as_ptr()
        } else {
            std::ptr::null()
        },
        Flags: OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_EXPLORER
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            },
        ..OPENFILENAMEW::default()
    };
    // SAFETY: every UTF-16 buffer and the dialog struct stay alive for the modal call.
    if unsafe {
        if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        }
    } != 0
    {
        let length = path.iter().position(|&v| v == 0).unwrap_or(path.len());
        return Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(
            &path[..length],
        ))));
    }
    // SAFETY: reads the calling thread's common-dialog error code.
    let error = unsafe { CommDlgExtendedError() };
    if error == 0 {
        Ok(None)
    } else {
        Err(format!("Windows file selection failed ({error:#x})."))
    }
}
#[cfg(not(windows))]
pub fn pick_rom(_: &Path) -> Result<Option<PathBuf>, String> {
    Err("Use the ROM library path on this platform.".into())
}

pub fn pick_save(folder: &Path, export: bool) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    {
        pick_file(folder, export, true)
    }
    #[cfg(not(windows))]
    {
        let _ = (folder, export);
        Err("Native save dialogs are unavailable on this platform".into())
    }
}
pub fn open_folder(folder: &Path) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(folder)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Open the configured ROM folder in your file manager".into())
    }
}

/// Dispatch an HTTP(S) link to the operating system's default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    validate_url(url)?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Com::{
            COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
        };
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let target: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
        // SAFETY: COM initialization is balanced on this thread when successful.
        let initialized = unsafe {
            CoInitializeEx(
                std::ptr::null(),
                (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
            )
        };
        // RPC_E_CHANGED_MODE means the caller already initialized COM in another apartment.
        if initialized < 0 && initialized != 0x80010106u32 as i32 {
            return Err(format!(
                "Windows browser initialization failed ({initialized:#x})"
            ));
        }
        // SAFETY: the UTF-16 strings remain alive; optional pointers are null; the verb is only open.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        } as isize;
        if initialized >= 0 {
            unsafe {
                CoUninitialize();
            }
        }
        shell_open_result(result)
    }
    #[cfg(not(windows))]
    {
        let executable = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let result = std::process::Command::new(executable)
            .arg(url)
            .status()
            .map_err(|e| format!("Cannot start the default browser: {e}"))?;
        if result.success() {
            Ok(())
        } else {
            Err(format!("The browser launcher exited with {result}"))
        }
    }
}
fn validate_url(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    if rest.is_none_or(|s| s.is_empty() || s.starts_with('/'))
        || url.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        Err("Only complete HTTP or HTTPS links can be opened".into())
    } else {
        Ok(())
    }
}
#[cfg(any(windows, test))]
fn shell_open_result(code: isize) -> Result<(), String> {
    if code > 32 {
        Ok(())
    } else {
        Err(format!(
            "Windows could not open the default browser (ShellExecute {code})"
        ))
    }
}

#[cfg(test)]
mod url_tests {
    use super::*;
    #[test]
    fn browser_errors_and_invalid_targets_are_explicit() {
        for code in [0, 2, 5, 8, 31, 32] {
            assert!(shell_open_result(code).is_err());
        }
        assert!(shell_open_result(33).is_ok());
        for url in [
            "",
            "https://",
            "file:///private",
            "javascript:alert(1)",
            "https:///",
            "https://example.com\0tail",
        ] {
            assert!(validate_url(url).is_err());
        }
        assert!(validate_url("https://tamatown.de").is_ok());
    }
    #[test]
    #[ignore = "Opens three browser tabs; run explicitly for native URL dispatch verification"]
    fn default_browser_opens_the_requested_community_links() {
        for url in [
            "https://tamatown.de",
            "https://github.com/Noktyros/RetroHatch",
            "https://discord.gg/w4zuzCKaZf",
        ] {
            open_url(url).unwrap();
        }
    }
}
