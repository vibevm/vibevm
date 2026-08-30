#![deny(unsafe_code)]
//! Safe author surface for VibeVM native extension ABI 1.
//!
//! Authors implement `fn(Context) -> Reply` and invoke [`vibe_extension!`].
//! The generated C boundary owns JSON conversion, panic containment, and the
//! exact response allocation/free pairing.

pub use vibe_wire::generated::native::e1::context::{
    Artifact, Context, Execution, Io, Project, Run, RunAgentMode, SlotTarget, World, WorldPackage,
};
pub use vibe_wire::generated::native::e1::manifest::{Manifest, ManifestExtension};
pub use vibe_wire::generated::native::e1::reply::{Reply, ReplyArtifact, ReplyStatus};

#[doc(hidden)]
pub use serde_json as __serde_json;

/// Exports one VibeVM native extension through the four-symbol ABI 1 surface.
///
/// The manifest is serialized once and remains valid for the library's
/// lifetime. Each successful invocation transfers one exact-length boxed byte
/// slice to the host, which must return it once through `vibe_ext_free`.
#[macro_export]
macro_rules! vibe_extension {
    (manifest = $manifest:expr, handler = $handler:path $(,)?) => {
        #[cfg(panic = "abort")]
        compile_error!(
            "vibe_extension! requires panic = \"unwind\"; remove panic = \"abort\" from the extension's active Cargo profile"
        );

        #[doc(hidden)]
        fn __vibe_ext_manifest_value() -> $crate::Manifest {
            $manifest
        }

        #[doc(hidden)]
        fn __vibe_ext_handle(context: $crate::Context) -> $crate::Reply {
            $handler(context)
        }

        #[doc(hidden)]
        #[allow(unsafe_code)]
        mod __vibe_ext_ffi {
            use std::ffi::{CString, c_char};
            use std::panic::{AssertUnwindSafe, catch_unwind};
            use std::ptr;
            use std::sync::OnceLock;

            static MANIFEST: OnceLock<CString> = OnceLock::new();

            fn initialize_outputs(
                response_ptr: *mut *mut u8,
                response_len: *mut usize,
            ) -> bool {
                if !response_ptr.is_null() {
                    // SAFETY: A non-null response slot is host-provided writable
                    // storage for one pointer under the C ABI contract.
                    unsafe { response_ptr.write(ptr::null_mut()) };
                }
                if !response_len.is_null() {
                    // SAFETY: A non-null length slot is host-provided writable
                    // storage for one usize under the C ABI contract.
                    unsafe { response_len.write(0) };
                }
                !response_ptr.is_null() && !response_len.is_null()
            }

            fn invoke(
                request_ptr: *const u8,
                request_len: usize,
                response_ptr: *mut *mut u8,
                response_len: *mut usize,
            ) -> i32 {
                if !initialize_outputs(response_ptr, response_len) || request_ptr.is_null() {
                    return 1;
                }

                let result = catch_unwind(AssertUnwindSafe(|| -> Option<Box<[u8]>> {
                    // SAFETY: The host retains a readable request allocation of
                    // exactly request_len bytes for the duration of this call.
                    let request = unsafe { std::slice::from_raw_parts(request_ptr, request_len) };
                    let context: $crate::Context =
                        $crate::__serde_json::from_slice(request).ok()?;
                    if context.envelope != 1 {
                        return None;
                    }
                    let reply = super::__vibe_ext_handle(context);
                    let bytes = $crate::__serde_json::to_vec(&reply).ok()?;
                    Some(bytes.into_boxed_slice())
                }));

                let Ok(Some(response)) = result else {
                    return 1;
                };
                let len = response.len();
                let raw = Box::into_raw(response).cast::<u8>();
                // SAFETY: Both output slots were checked and initialized before
                // request handling. Ownership is published only after success.
                unsafe {
                    response_ptr.write(raw);
                    response_len.write(len);
                }
                0
            }

            fn free(ptr: *mut u8, len: usize) {
                if ptr.is_null() {
                    return;
                }
                let slice = ptr::slice_from_raw_parts_mut(ptr, len);
                // SAFETY: Successful invoke returned this pointer and exact
                // boxed-slice length, and the host returns that pair once.
                drop(unsafe { Box::from_raw(slice) });
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn vibe_ext_abi() -> u32 {
                1
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn vibe_ext_manifest() -> *const c_char {
                MANIFEST
                    .get_or_init(|| {
                        let json = $crate::__serde_json::to_vec(&super::__vibe_ext_manifest_value())
                            .expect("generated Manifest serializes to JSON");
                        CString::new(json).expect("serialized manifest JSON contains no NUL byte")
                    })
                    .as_ptr()
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn vibe_ext_invoke(
                request_ptr: *const u8,
                request_len: usize,
                response_ptr: *mut *mut u8,
                response_len: *mut usize,
            ) -> i32 {
                invoke(request_ptr, request_len, response_ptr, response_len)
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn vibe_ext_free(ptr: *mut u8, len: usize) {
                free(ptr, len);
            }
        }

        #[doc(hidden)]
        pub use __vibe_ext_ffi::{
            vibe_ext_abi, vibe_ext_free, vibe_ext_invoke, vibe_ext_manifest,
        };
    };
}
