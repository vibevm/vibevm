//! Native metadata from a retained directory, without exporting its handle.

use super::Project;

impl Project {
    /// Maximum filename component length on this held directory's volume, in
    /// Windows WCHAR (UTF-16 code) units. This is not a total pathname limit.
    /// A failed or zero-valued query is unavailable, never a guessed default.
    #[allow(unsafe_code)]
    pub fn windows_maximum_component_length(&self) -> std::io::Result<u32> {
        use std::{ffi::c_void, os::windows::io::AsRawHandle, ptr};

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetVolumeInformationByHandleW(
                file: *mut c_void,
                volume_name: *mut u16,
                volume_name_size: u32,
                serial_number: *mut u32,
                maximum_component_length: *mut u32,
                flags: *mut u32,
                filesystem_name: *mut u16,
                filesystem_name_size: u32,
            ) -> i32;
        }

        let mut maximum = 0;
        // SAFETY: self retains the live directory handle throughout this call.
        // The sole requested output points to an initialized writable DWORD;
        // all other optional outputs are NULL with zero buffer lengths. The
        // function neither owns nor closes the handle. Signature/units:
        // https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationbyhandlew
        let result = unsafe {
            GetVolumeInformationByHandleW(
                self.root.as_raw_handle(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                &mut maximum,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
            )
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if maximum == 0 {
            return Err(std::io::Error::other(
                "volume returned a zero maximum component length",
            ));
        }
        Ok(maximum)
    }
}
