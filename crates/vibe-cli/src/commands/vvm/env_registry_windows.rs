//! The registry calls behind the Windows user environment. Every value is
//! UTF-16 on the wire; the conversions go through `std::os::windows::ffi`
//! so a name or value outside ASCII survives both directions.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SAM_FLAGS, REG_SZ,
    REG_VALUE_TYPE, RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};

use super::{RawUserVar, RegistryValueKind};

const SUBKEY: &str = "Environment";

/// How long one hung top-level window may hold the broadcast, in
/// milliseconds; the value Explorer and the System dialog use.
const BROADCAST_TIMEOUT_MS: u32 = 5000;

/// A NUL-terminated UTF-16 copy of `text` for a `PCWSTR` argument.
fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

fn os_error(code: WIN32_ERROR) -> std::io::Error {
    std::io::Error::from_raw_os_error(code as i32)
}

/// An open `HKCU\Environment` handle, closed exactly once on drop.
struct Key(HKEY);

impl Key {
    /// Open the key for `access`; `None` when the profile has no
    /// `Environment` key at all, which reads as "no values".
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API for the Windows registry; the subkey is a NUL-terminated buffer that outlives the call and the returned handle is owned by this guard (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
    )]
    fn open(access: REG_SAM_FLAGS) -> Result<Option<Key>> {
        let subkey = wide(SUBKEY);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: `subkey` is NUL-terminated and alive for the call; `handle`
        // is a valid out-pointer and is only used when the call succeeds.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, access, &mut handle) };
        match status {
            ERROR_SUCCESS => Ok(Some(Key(handle))),
            ERROR_FILE_NOT_FOUND => Ok(None),
            other => bail!("opening HKCU\\Environment: {}", os_error(other)),
        }
    }

    /// Open the key for writing, creating it when the profile lacks it, as
    /// the System dialog does.
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API for the Windows registry; the subkey is a NUL-terminated buffer that outlives the call and the returned handle is owned by this guard (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
    )]
    fn create() -> Result<Key> {
        let subkey = wide(SUBKEY);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: as in `open`; the handle is only used on success.
        let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey.as_ptr(), &mut handle) };
        if status != ERROR_SUCCESS {
            bail!(
                "opening HKCU\\Environment for writing: {}",
                os_error(status)
            );
        }
        Ok(Key(handle))
    }
}

impl Drop for Key {
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: closes the registry handle this guard owns, exactly once (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
    )]
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful open and is closed once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

/// One `RegQueryValueExW` into `buffer`; `bytes` carries the buffer's size
/// in and the value's size out.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: Rust has no standard-library API for the Windows registry; the name is NUL-terminated and `bytes` never exceeds the buffer the call writes into (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
)]
fn query(
    key: &Key,
    name: &[u16],
    kind: &mut REG_VALUE_TYPE,
    buffer: &mut [u16],
    bytes: &mut u32,
) -> WIN32_ERROR {
    // SAFETY: `name` is NUL-terminated; `buffer` holds at least `*bytes`
    // bytes, which the caller derives from its length.
    unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            kind,
            buffer.as_mut_ptr().cast(),
            bytes,
        )
    }
}

/// Read one raw `HKCU\Environment` value; `None` when it is absent. The
/// registry API never expands `%VAR%`, so a `REG_EXPAND_SZ` PATH comes back
/// exactly as stored.
pub(crate) fn get_user_var_raw(name: &str) -> Result<Option<RawUserVar>> {
    let Some(key) = Key::open(KEY_QUERY_VALUE)? else {
        return Ok(None);
    };
    let name_w = wide(name);
    let mut buffer: Vec<u16> = vec![0; 512];
    loop {
        let mut kind: REG_VALUE_TYPE = 0;
        let mut bytes = u32::try_from(buffer.len() * 2)?;
        match query(&key, &name_w, &mut kind, &mut buffer, &mut bytes) {
            ERROR_SUCCESS => {
                let kind = match kind {
                    REG_SZ => RegistryValueKind::String,
                    REG_EXPAND_SZ => RegistryValueKind::ExpandString,
                    other => bail!(
                        "HKCU\\Environment value `{name}` has registry type {other}, not a string"
                    ),
                };
                let mut units = &buffer[..(bytes as usize / 2).min(buffer.len())];
                // The stored terminator is optional and sometimes doubled.
                while let [rest @ .., 0] = units {
                    units = rest;
                }
                let value = OsString::from_wide(units).to_string_lossy().into_owned();
                return Ok(Some(RawUserVar { value, kind }));
            }
            ERROR_FILE_NOT_FOUND => return Ok(None),
            // The value grew or exceeded the first guess: retry at its size.
            ERROR_MORE_DATA => buffer.resize((bytes as usize).div_ceil(2) + 1, 0),
            other => bail!(
                "reading HKCU\\Environment value `{name}`: {}",
                os_error(other)
            ),
        }
    }
}

/// Write one raw `HKCU\Environment` value with its kind, then broadcast.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: Rust has no standard-library API for the Windows registry; name and data are NUL-terminated buffers alive for the call and the byte count is their exact length (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
)]
pub(crate) fn set_user_var_raw(name: &str, value: &str, kind: RegistryValueKind) -> Result<()> {
    let key = Key::create()?;
    let name_w = wide(name);
    let data = wide(value);
    let bytes = u32::try_from(data.len() * 2)?;
    let kind = match kind {
        RegistryValueKind::String => REG_SZ,
        RegistryValueKind::ExpandString => REG_EXPAND_SZ,
    };
    // SAFETY: `name_w` and `data` are NUL-terminated and alive for the call;
    // `bytes` is `data`'s length in bytes, terminator included, as the API
    // requires for string types.
    let status =
        unsafe { RegSetValueExW(key.0, name_w.as_ptr(), 0, kind, data.as_ptr().cast(), bytes) };
    if status != ERROR_SUCCESS {
        bail!(
            "writing HKCU\\Environment value `{name}`: {}",
            os_error(status)
        );
    }
    drop(key);
    broadcast();
    Ok(())
}

/// Delete one `HKCU\Environment` value, then broadcast the change. An
/// absent value is already the requested state.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: Rust has no standard-library API for the Windows registry; the name is a NUL-terminated buffer alive for the call (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
)]
pub(crate) fn delete_user_var_raw(name: &str) -> Result<()> {
    let Some(key) = Key::open(KEY_SET_VALUE)? else {
        return Ok(());
    };
    let name_w = wide(name);
    // SAFETY: `name_w` is NUL-terminated and alive for the call.
    let status = unsafe { RegDeleteValueW(key.0, name_w.as_ptr()) };
    match status {
        ERROR_SUCCESS => {}
        ERROR_FILE_NOT_FOUND => return Ok(()),
        other => bail!(
            "deleting HKCU\\Environment value `{name}`: {}",
            os_error(other)
        ),
    }
    drop(key);
    broadcast();
    Ok(())
}

/// Tell running top-level windows (Explorer first) that the environment
/// changed, so terminals started from them inherit the new values. The
/// write already landed; a window that ignores or times out the message
/// only delays its own refresh, so the result is not an error.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: Rust has no standard-library API for window messages; the parameter is a NUL-terminated buffer alive for the whole bounded call (spec://org.vibevm.core/vibevm/common/PROP-019#path)."
)]
fn broadcast() {
    let area = wide(SUBKEY);
    let mut result: usize = 0;
    // SAFETY: `area` is NUL-terminated and outlives the call, which returns
    // within the timeout; `result` is a valid out-pointer.
    let _ = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            area.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            BROADCAST_TIMEOUT_MS,
            &mut result,
        )
    };
}

/// A live round trip through the real `HKCU\Environment`. It mutates the
/// user's registry, so it runs only when `VIBE_TEST_HKCU=1` is set.
#[cfg(test)]
mod tests {
    use super::*;

    const SCRATCH: &str = "VIBE_TEST_HKCU_SCRATCH";

    #[test]
    fn a_scratch_value_round_trips_with_its_kind() {
        if std::env::var_os("VIBE_TEST_HKCU").as_deref() != Some(OsStr::new("1")) {
            return;
        }
        set_user_var_raw(
            SCRATCH,
            r"%USERPROFILE%\vibe-é",
            RegistryValueKind::ExpandString,
        )
        .unwrap();
        let read = get_user_var_raw(SCRATCH).unwrap().expect("written");
        assert_eq!(read.value, r"%USERPROFILE%\vibe-é", "read back unexpanded");
        assert_eq!(read.kind, RegistryValueKind::ExpandString);
        set_user_var_raw(SCRATCH, "plain", RegistryValueKind::String).unwrap();
        let read = get_user_var_raw(SCRATCH).unwrap().expect("rewritten");
        assert_eq!(
            (read.value.as_str(), read.kind),
            ("plain", RegistryValueKind::String)
        );
        delete_user_var_raw(SCRATCH).unwrap();
        assert!(get_user_var_raw(SCRATCH).unwrap().is_none());
        delete_user_var_raw(SCRATCH).unwrap();
    }
}
