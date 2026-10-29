//! Operating-system specific pieces (Windows and macOS). Everything else in the app is shared.

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

// ---------- macOS: Core Text font manager ----------

#[cfg(target_os = "macos")]
mod mac {
    use core_foundation::base::TCFType;
    use core_foundation::error::CFErrorRef;
    use core_foundation::url::{CFURLRef, CFURL};

    /// The font is available to every app until logout.
    pub const SCOPE_SESSION: u32 = 3;
    /// The font is available to every app, also after restarting.
    pub const SCOPE_USER: u32 = 2;

    #[link(name = "CoreText", kind = "framework")]
    extern "C" {
        fn CTFontManagerRegisterFontsForURL(url: CFURLRef, scope: u32, error: *mut CFErrorRef) -> bool;
        fn CTFontManagerUnregisterFontsForURL(url: CFURLRef, scope: u32, error: *mut CFErrorRef) -> bool;
    }

    fn url(path: &str) -> Option<CFURL> {
        CFURL::from_path(path, false)
    }

    pub fn register(path: &str, scope: u32) -> bool {
        let Some(u) = url(path) else { return false };
        unsafe { CTFontManagerRegisterFontsForURL(u.as_concrete_TypeRef(), scope, std::ptr::null_mut()) }
    }

    pub fn unregister(path: &str, scope: u32) -> bool {
        let Some(u) = url(path) else { return false };
        unsafe { CTFontManagerUnregisterFontsForURL(u.as_concrete_TypeRef(), scope, std::ptr::null_mut()) }
    }
}

/// Makes font files available to other apps (Illustrator, Affinity…) without
/// installing them. Returns the paths that failed.
#[cfg(target_os = "macos")]
pub fn activate(paths: &[String]) -> Vec<String> {
    let mut failed = Vec::new();
    for p in paths {
        // Session scope first (gone at logout, like on Windows); the app re-activates
        // fonts when it starts. If that's refused, use the user scope. A font that is
        // already registered is refused too, and counts as active.
        if !mac::register(p, mac::SCOPE_SESSION) && !mac::register(p, mac::SCOPE_USER) && !std::path::Path::new(p).exists() {
            failed.push(p.clone());
        }
    }
    failed
}

/// Removes fonts added by `activate`. Returns the paths that failed.
#[cfg(target_os = "macos")]
pub fn deactivate(paths: &[String]) -> Vec<String> {
    let mut failed = Vec::new();
    for p in paths {
        let a = mac::unregister(p, mac::SCOPE_SESSION);
        let b = mac::unregister(p, mac::SCOPE_USER);
        if !a && !b {
            failed.push(p.clone());
        }
    }
    failed
}

// Other systems (Linux, used only for development): nothing to do.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn activate(_paths: &[String]) -> Vec<String> {
    Vec::new()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn deactivate(_paths: &[String]) -> Vec<String> {
    Vec::new()
}

/// Runs an ExtendScript file in Illustrator through its COM interface and returns what
/// the script returned. Illustrator is started if it isn't running.
#[cfg(windows)]
pub fn run_illustrator_script(script: &std::path::Path) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let p = script.to_string_lossy().replace('\'', "''");
    let cmd = format!(
        "$ErrorActionPreference='Stop'; try {{ $ai = New-Object -ComObject Illustrator.Application }} catch {{ Write-Output 'ERROR:Illustrator is not installed or could not be started'; exit }}; $r = $ai.DoJavaScriptFile('{p}'); Write-Output $r"
    );
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("could not start PowerShell: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("no answer from Illustrator").to_string();
        return Err(err);
    }
    Ok(stdout)
}

/// Runs an ExtendScript file in Illustrator through AppleScript and returns what the
/// script returned. Illustrator is started if it isn't running. The first time, macOS
/// asks whether Buni Font Manager may control Illustrator.
#[cfg(target_os = "macos")]
pub fn run_illustrator_script(script: &std::path::Path) -> Result<String, String> {
    let p = script.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    let apple = format!(
        "set code to read (POSIX file \"{p}\")\nwith timeout of 900 seconds\ntell application id \"com.adobe.illustrator\"\nactivate\nset r to do javascript code\nend tell\nend timeout\nreturn r"
    );
    let out = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", &apple])
        .output()
        .map_err(|e| format!("could not start osascript: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || stdout.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let err = if err.contains("-1743") || err.contains("Not authorized") {
            "macOS didn't allow Buni Font Manager to control Illustrator. Allow it in System Settings > Privacy & Security > Automation".to_string()
        } else if err.contains("-1728") || err.contains("Can’t get application") || err.contains("Unable to find application") {
            "Illustrator is not installed".to_string()
        } else if err.is_empty() {
            "no answer from Illustrator".to_string()
        } else {
            err
        };
        return Err(err);
    }
    Ok(stdout)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn run_illustrator_script(_script: &std::path::Path) -> Result<String, String> {
    Err("Illustrator files can only be made on Windows and macOS".into())
}
