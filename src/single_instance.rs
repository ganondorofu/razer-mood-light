use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;

/// Claims a named OS mutex so only one copy of this app can run at a time.
/// Returns false if another instance already holds it — the caller should
/// exit immediately in that case. The mutex handle is intentionally leaked
/// for the process lifetime; Windows releases it automatically on exit.
pub fn acquire() -> bool {
    let name: Vec<u16> = "Global\\ClaudeMoodLightSingleInstance"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}
