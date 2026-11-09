//! Native ROM selection; no firmware downloading or automatic file execution.
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub fn pick_rom(folder: &Path) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let filter: Vec<u16> = "Tamagotchi ROM (*.bin;*.rom)\0*.bin;*.rom\0All files\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = "Open user-provided Tamagotchi ROM"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let initial: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut path = vec![0u16; 32768];
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFile: path.as_mut_ptr(),
        nMaxFile: path.len() as u32,
        lpstrFilter: filter.as_ptr(),
        lpstrTitle: title.as_ptr(),
        lpstrInitialDir: initial.as_ptr(),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER,
        ..OPENFILENAMEW::default()
    };
    // SAFETY: every UTF-16 buffer and the dialog struct stay alive for the modal call.
    if unsafe { GetOpenFileNameW(&mut dialog) } != 0 {
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
