// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Native window chrome plus the renderer's exact client pixels, never desktop pixels.
use super::Image;
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{Dwm::*, Gdi::*},
    UI::{Input::KeyboardAndMouse::GetActiveWindow, WindowsAndMessaging::*},
};
#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(window: HWND, dc: HDC, flags: u32) -> i32;
}
pub fn window() -> Result<usize, String> {
    // SAFETY: query the calling UI thread's window and verify ownership before capture.
    unsafe {
        let h = GetActiveWindow();
        let mut pid = 0;
        if h.is_null()
            || GetWindowThreadProcessId(h, &mut pid) == 0
            || pid != std::process::id()
            || IsIconic(h) != 0
        {
            return Err("The RetroHatch window must be visible to take a full screenshot".into());
        }
        Ok(h as usize)
    }
}
pub fn compose(window: usize, client: Image) -> Result<Image, String> {
    let h = window as HWND;
    let mut pid = 0;
    // SAFETY: all rectangles, bitmap buffers and handles remain owned until GDI completes.
    unsafe {
        if h.is_null() || GetWindowThreadProcessId(h, &mut pid) == 0 || pid != std::process::id() {
            return Err("Screenshot window is no longer available".into());
        }
        let mut bounds = RECT::default();
        let mut area = RECT::default();
        let mut origin = POINT::default();
        if GetWindowRect(h, &mut bounds) == 0
            || GetClientRect(h, &mut area) == 0
            || ClientToScreen(h, &mut origin) == 0
        {
            return Err("Cannot query screenshot window bounds".into());
        }
        let w = bounds.right - bounds.left;
        let ht = bounds.bottom - bounds.top;
        if w <= 0
            || ht <= 0
            || (w as usize)
                .checked_mul(ht as usize)
                .and_then(|v| v.checked_mul(4))
                .is_none_or(|v| v > 64 * 1024 * 1024)
            || area.right as usize != client.width
            || area.bottom as usize != client.height
        {
            return Err("Window changed size during capture; please try again".into());
        }
        let source = GetWindowDC(h);
        if source.is_null() {
            return Err("Cannot acquire the screenshot window surface".into());
        }
        let dc = CreateCompatibleDC(source);
        if dc.is_null() {
            ReleaseDC(h, source);
            return Err("Cannot create the screenshot surface".into());
        }
        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -ht,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            source,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if bitmap.is_null() || bits.is_null() {
            DeleteDC(dc);
            ReleaseDC(h, source);
            return Err("Cannot allocate screenshot pixels".into());
        }
        std::ptr::write_bytes(bits.cast::<u8>(), 0, w as usize * ht as usize * 4);
        let previous = SelectObject(dc, bitmap);
        if previous.is_null() || previous as isize == -1 {
            DeleteObject(bitmap);
            DeleteDC(dc);
            ReleaseDC(h, source);
            return Err("Cannot select the screenshot bitmap".into());
        }
        let printed = PrintWindow(h, dc, 2);
        GdiFlush();
        let mut rgb = Vec::with_capacity(w as usize * ht as usize * 3);
        if printed != 0 {
            for p in std::slice::from_raw_parts(bits.cast::<u8>(), w as usize * ht as usize * 4)
                .chunks_exact(4)
            {
                rgb.extend_from_slice(&[p[2], p[1], p[0]]);
            }
        }
        SelectObject(dc, previous);
        DeleteObject(bitmap);
        DeleteDC(dc);
        ReleaseDC(h, source);
        if printed == 0 {
            return Err("Windows could not capture this application window".into());
        }
        let x = origin.x - bounds.left;
        let y = origin.y - bounds.top;
        if x < 0
            || y < 0
            || x as usize + client.width > w as usize
            || y as usize + client.height > ht as usize
        {
            return Err("Invalid client bounds during screenshot".into());
        }
        // OpenGL surfaces are not reliably included by PrintWindow. Overlay the actual GPU readback.
        for row in 0..client.height {
            let start = ((y as usize + row) * w as usize + x as usize) * 3;
            rgb[start..start + client.width * 3]
                .copy_from_slice(&client.rgb[row * client.width * 3..(row + 1) * client.width * 3]);
        }
        let mut visible = bounds;
        if DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut visible as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        ) < 0
        {
            visible = bounds;
        }
        let left = (visible.left - bounds.left).clamp(0, w);
        let top = (visible.top - bounds.top).clamp(0, ht);
        let right = (visible.right - bounds.left).clamp(left, w);
        let bottom = (visible.bottom - bounds.top).clamp(top, ht);
        let cw = (right - left) as usize;
        let ch = (bottom - top) as usize;
        let mut cropped = Vec::with_capacity(cw * ch * 3);
        for row in top..bottom {
            let start = (row as usize * w as usize + left as usize) * 3;
            cropped.extend_from_slice(&rgb[start..start + cw * 3]);
        }
        Image::new(cw, ch, cropped)
    }
}
