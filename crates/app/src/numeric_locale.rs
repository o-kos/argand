//! Read the numeric region without changing the process-wide C locale.

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn locale() -> String {
    let name = ["LC_ALL", "LC_NUMERIC", "LANG"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| "C".into());
    normalize(&name)
}

#[cfg(target_os = "windows")]
pub fn locale() -> String {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, length: i32) -> i32;
    }
    let mut name = [0u16; 85];
    // The API writes at most the supplied buffer length, including its terminator.
    let length = unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) };
    if length > 1 && length as usize <= name.len() {
        String::from_utf16_lossy(&name[..length as usize - 1])
    } else {
        "en-US".into()
    }
}

#[cfg(target_os = "macos")]
pub fn locale() -> String {
    use std::ffi::{CStr, c_char, c_void};
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFLocaleCopyCurrent() -> *const c_void;
        fn CFLocaleGetIdentifier(locale: *const c_void) -> *const c_void;
        fn CFStringGetCString(
            value: *const c_void,
            buffer: *mut c_char,
            size: isize,
            encoding: u32,
        ) -> bool;
        fn CFRelease(value: *const c_void);
    }
    let mut name = [0 as c_char; 128];
    // Copy owns the locale; its identifier stays borrowed until CFRelease.
    let copied = unsafe {
        let locale = CFLocaleCopyCurrent();
        if locale.is_null() {
            return "en-US".into();
        }
        let identifier = CFLocaleGetIdentifier(locale);
        let copied = !identifier.is_null()
            && CFStringGetCString(
                identifier,
                name.as_mut_ptr(),
                name.len() as isize,
                0x08000100,
            );
        CFRelease(locale);
        copied
    };
    if copied {
        // Successful conversion guarantees a NUL-terminated UTF-8 string.
        normalize(&unsafe { CStr::from_ptr(name.as_ptr()) }.to_string_lossy())
    } else {
        "en-US".into()
    }
}

#[cfg(not(target_os = "windows"))]
fn normalize(name: &str) -> String {
    let (base, keywords) = name.split_once('@').unwrap_or((name, ""));
    let mut locale = base.split('.').next().unwrap_or("C").replace('_', "-");
    if let Some(numbering) = keywords
        .split(';')
        .find_map(|entry| entry.strip_prefix("numbers="))
    {
        locale.push_str("-u-nu-");
        locale.push_str(numbering);
    }
    locale
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;
    #[test]
    fn os_locale_identifiers_keep_numeric_region_and_numbering() {
        assert_eq!(normalize("ru_RU.UTF-8"), "ru-RU");
        assert_eq!(normalize("C.UTF-8"), "C");
        assert_eq!(normalize("de_DE@calendar=gregorian"), "de-DE");
        assert_eq!(
            normalize("ar_EG@calendar=gregorian;numbers=latn"),
            "ar-EG-u-nu-latn"
        );
    }
}
