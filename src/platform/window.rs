//! Native window opacity via the Windows API.

use crate::*;

pub(crate) fn set_window_opacity(
    window: &impl raw_window_handle::HasWindowHandle,
    alpha: Option<u8>,
) -> Result<(), String> {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetLayeredWindowAttributes, GetWindowLongPtrW, SetLayeredWindowAttributes,
        SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
    };
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Kein Windows-Fensterhandle".into());
    };
    let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
    // Layered-window alpha is composed by Windows, independently of OpenGL's framebuffer alpha.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if let Some(alpha) = alpha {
            if style & WS_EX_LAYERED as isize == 0 {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
                if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_LAYERED as isize == 0 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
            }
            if SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let (mut actual_alpha, mut flags, mut color) = (0, 0, 0);
            if GetLayeredWindowAttributes(hwnd, &mut color, &mut actual_alpha, &mut flags) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            if actual_alpha != alpha || flags & LWA_ALPHA == 0 {
                return Err("Windows hat die Fensterdeckkraft nicht übernommen".into());
            }
        } else if style & WS_EX_LAYERED as isize != 0 {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_LAYERED as isize));
            if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_LAYERED as isize != 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
    }
    Ok(())
}
