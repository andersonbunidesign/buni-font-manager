//! Operating-system specific pieces. Everything else in the app is shared, so a
//! macOS version only needs a macOS implementation of this file.

/// Makes font files available to other apps (Illustrator, Affinity…) for this
/// Windows session, without installing them. Returns the paths that failed.
#[cfg(windows)]
pub fn activate(paths: &[String]) -> Vec<String> {
    use windows_sys::Win32::Graphics::Gdi::AddFontResourceExW;
    let mut failed = Vec::new();
    for p in paths {
        let wide = to_wide(p);
        // 0 = public font (not FR_PRIVATE), so every app can use it.
        let added = unsafe { AddFontResourceExW(wide.as_ptr(), 0, std::ptr::null()) };
        if added == 0 {
            failed.push(p.clone());
        }
    }
    if !paths.is_empty() {
        notify_font_change();
    }
    failed
}

/// Removes fonts added by `activate`. Returns the paths that failed.
#[cfg(windows)]
pub fn deactivate(paths: &[String]) -> Vec<String> {
    use windows_sys::Win32::Graphics::Gdi::RemoveFontResourceExW;
    let mut failed = Vec::new();
    for p in paths {
        let wide = to_wide(p);
        // A font can be added more than once; remove until Windows says it's gone.
        let mut removed_any = false;
        for _ in 0..8 {
            let ok = unsafe { RemoveFontResourceExW(wide.as_ptr(), 0, std::ptr::null()) };
            if ok == 0 {
                break;
            }
            removed_any = true;
        }
        if !removed_any {
            failed.push(p.clone());
        }
    }
    if !paths.is_empty() {
        notify_font_change();
    }
    failed
}

/// Tells running apps that the font list changed, so they pick up the change without restarting.
#[cfg(windows)]
fn notify_font_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_FONTCHANGE};
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0, SMTO_ABORTIFHUNG, 1000, &mut result);
    }
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

// Other systems: not implemented yet (the macOS version will fill these in).
#[cfg(not(windows))]
pub fn activate(_paths: &[String]) -> Vec<String> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn deactivate(_paths: &[String]) -> Vec<String> {
    Vec::new()
}
