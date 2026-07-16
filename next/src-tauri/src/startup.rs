const TITLE: &str = "Streamlink Twitch GUI";

#[derive(Debug)]
pub struct StartupFailure {
    message: String,
}

impl StartupFailure {
    pub fn new(cause: impl std::fmt::Display) -> Self {
        let cause = cause.to_string().replace('\0', " ");
        Self {
            message: format!(
                "The application could not start.\n\n{cause}\n\nCheck the installation and settings, then try again."
            ),
        }
    }

    pub fn title(&self) -> &'static str {
        TITLE
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

pub fn report(failure: &StartupFailure) {
    eprintln!("{}: {}", failure.title(), failure.message());
    show_native_error(failure);
}

#[cfg(windows)]
fn show_native_error(failure: &StartupFailure) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

    let title = std::ffi::OsStr::new(failure.title())
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let message = std::ffi::OsStr::new(failure.message())
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_native_error(_failure: &StartupFailure) {}
