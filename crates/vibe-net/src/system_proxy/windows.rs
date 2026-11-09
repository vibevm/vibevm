//! The Windows reader behind the [`WindowsProxySource`] seam: the WinHTTP
//! calls that expose the logged-in user's proxy configuration and the two
//! registry reads that stand in when they answer nothing. Every value is
//! UTF-16 on the wire; the conversions go through `std::os::windows::ffi`
//! so a PAC URL outside ASCII survives.

use std::ffi::{OsStr, OsString, c_void};
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, GlobalFree, WIN32_ERROR,
};
use windows_sys::Win32::Networking::WinHttp::{
    WINHTTP_AUTO_DETECT_TYPE_DHCP, WINHTTP_AUTO_DETECT_TYPE_DNS_A,
    WINHTTP_CURRENT_USER_IE_PROXY_CONFIG, WinHttpDetectAutoProxyConfigUrl,
    WinHttpGetIEProxyConfigForCurrentUser,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_BINARY, RRF_RT_REG_EXPAND_SZ,
    RRF_RT_REG_SZ, RegGetValueW,
};

use vibe_core::net::system_proxy::{IeProxyConfig, POLICY_SUBKEY, USER_SUBKEY, WindowsProxySource};

/// The connection whose settings a `Connections` blob holds for the LAN;
/// per-dial-up entries live under their own value names.
const CONNECTIONS_SUBKEY: &str = r"Connections";

/// This host's own settings, read now.
#[derive(Debug)]
pub(crate) struct Reader;

/// A NUL-terminated UTF-16 copy of `text` for a `PCWSTR` argument.
fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

fn os_error(code: WIN32_ERROR) -> String {
    std::io::Error::from_raw_os_error(code as i32).to_string()
}

/// One registry value as bytes: `None` when the key or the value is
/// absent, `Err` when it is there and could not be read. `flags` is the
/// `RRF_RT_*` type restriction, so a value of the wrong kind reads as a
/// failure naming it rather than as nonsense.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: Rust has no standard-library API for the Windows registry; the subkey and value names are NUL-terminated buffers that outlive the call, and `bytes` never exceeds the buffer the call writes into (spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system)."
)]
fn read_value(root: HKEY, subkey: &str, name: &str, flags: u32) -> Result<Option<Vec<u8>>, String> {
    let subkey_w = wide(subkey);
    let name_w = wide(name);
    // Ask for the size, then read. The probe passes no buffer, which
    // `RegGetValueW` answers with ERROR_SUCCESS and the size — not
    // ERROR_MORE_DATA, which is what a buffer too small for a value that
    // grew between the two calls answers; the loop then asks once more.
    let mut buffer: Vec<u8> = Vec::new();
    let mut probing = true;
    for _ in 0..4 {
        let mut bytes = u32::try_from(buffer.len()).map_err(|error| error.to_string())?;
        let data = if probing {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr().cast::<c_void>()
        };
        // SAFETY: both names are NUL-terminated and alive for the call;
        // `data` points at `bytes` writable bytes (or is null, which asks
        // for the size only), and `bytes` is a valid out-pointer.
        let status = unsafe {
            RegGetValueW(
                root,
                subkey_w.as_ptr(),
                name_w.as_ptr(),
                flags,
                std::ptr::null_mut(),
                data,
                &mut bytes,
            )
        };
        match status {
            // The probe: the value is there and `bytes` is now its size.
            ERROR_SUCCESS if probing => {
                if bytes == 0 {
                    return Ok(Some(Vec::new()));
                }
                buffer = vec![0; bytes as usize];
                probing = false;
            }
            ERROR_SUCCESS => {
                buffer.truncate(bytes as usize);
                return Ok(Some(buffer));
            }
            ERROR_FILE_NOT_FOUND => return Ok(None),
            ERROR_MORE_DATA => {
                buffer = vec![0; bytes.max(1) as usize];
                probing = false;
            }
            other => return Err(os_error(other)),
        }
    }
    Err("the value kept growing between reads".to_owned())
}

/// One `REG_SZ` / `REG_EXPAND_SZ` value as text.
fn read_string(root: HKEY, subkey: &str, name: &str) -> Result<Option<String>, String> {
    let Some(bytes) = read_value(root, subkey, name, RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ)? else {
        return Ok(None);
    };
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .take_while(|unit| *unit != 0)
        .collect();
    let text = OsString::from_wide(&units).to_string_lossy().into_owned();
    Ok(Some(text))
}

/// A `PWSTR` WinHTTP allocated: its text, then the allocation released.
///
/// WinHTTP allocates every string it hands back with `GlobalAlloc`, and
/// its documentation requires the caller to free each one with
/// `GlobalFree`; a vibe that only read them would leak a little on every
/// operation that starts px.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: the pointer came from a successful WinHTTP call that documents it as a NUL-terminated `GlobalAlloc` string the caller frees; it is read once and freed exactly once (spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system)."
)]
fn take_wide(pointer: *mut u16) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: WinHTTP documents the pointer as a NUL-terminated wide
    // string; the length is measured up to that terminator and the slice
    // is only read while the allocation is still alive.
    let text = unsafe {
        let mut length = 0usize;
        while *pointer.add(length) != 0 {
            length += 1;
        }
        OsString::from_wide(std::slice::from_raw_parts(pointer, length))
            .to_string_lossy()
            .into_owned()
    };
    // SAFETY: the same allocation, freed exactly once, after its last read.
    unsafe { GlobalFree(pointer.cast()) };
    let text = text.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

impl WindowsProxySource for Reader {
    fn policy_auto_config_url(&self, machine: bool) -> Result<Option<String>, String> {
        let root = if machine {
            HKEY_LOCAL_MACHINE
        } else {
            HKEY_CURRENT_USER
        };
        read_string(root, POLICY_SUBKEY, "AutoConfigURL")
    }

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: `WinHttpGetIEProxyConfigForCurrentUser` is the only API that exposes the logged-in user's proxy configuration (the `Connections` blob included); the out-structure is a local this call fills and each string it allocated is freed once (spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system)."
    )]
    fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String> {
        let mut raw = WINHTTP_CURRENT_USER_IE_PROXY_CONFIG::default();
        // SAFETY: `raw` is a valid, fully initialised out-structure that
        // outlives the call; the strings it leaves are freed by `take_wide`.
        let ok = unsafe { WinHttpGetIEProxyConfigForCurrentUser(&mut raw) };
        if ok == 0 {
            let code = std::io::Error::last_os_error();
            return Err(format!("{code}"));
        }
        Ok(Some(IeProxyConfig {
            auto_detect: raw.fAutoDetect != 0,
            auto_config_url: take_wide(raw.lpszAutoConfigUrl),
            proxy: take_wide(raw.lpszProxy),
            proxy_bypass: take_wide(raw.lpszProxyBypass),
        }))
    }

    fn auto_config_url_value(&self) -> Result<Option<String>, String> {
        read_string(HKEY_CURRENT_USER, USER_SUBKEY, "AutoConfigURL")
    }

    fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String> {
        read_value(
            HKEY_CURRENT_USER,
            &format!("{USER_SUBKEY}\\{CONNECTIONS_SUBKEY}"),
            "DefaultConnectionSettings",
            RRF_RT_REG_BINARY,
        )
    }

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: `WinHttpDetectAutoProxyConfigUrl` is the only WPAD resolver Windows exposes; the out-pointer is a local and the string it allocates is freed once (spec://org.vibevm.core/vibevm/common/PROP-068#proxy-system)."
    )]
    fn detect_auto_proxy_url(&self) -> Result<Option<String>, String> {
        let mut url: *mut u16 = std::ptr::null_mut();
        // SAFETY: `url` is a valid out-pointer that outlives the call; the
        // string it leaves on success is freed by `take_wide`.
        let ok = unsafe {
            WinHttpDetectAutoProxyConfigUrl(
                WINHTTP_AUTO_DETECT_TYPE_DHCP | WINHTTP_AUTO_DETECT_TYPE_DNS_A,
                &mut url,
            )
        };
        if ok == 0 {
            // No WPAD answer is the ordinary case on a machine that does
            // not run one; it is reported as "no URL found", not a failure.
            let code = std::io::Error::last_os_error();
            return match code.raw_os_error() {
                // ERROR_WINHTTP_AUTODETECTION_FAILED
                Some(12180) => Ok(None),
                _ => Err(format!("{code}")),
            };
        }
        Ok(take_wide(url))
    }
}
