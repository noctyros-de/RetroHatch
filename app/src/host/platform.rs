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
