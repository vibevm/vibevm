use std::ffi::c_char;
use std::ptr::NonNull;
use std::sync::Arc;

use libloading::Library;

use crate::error::NativeLoadError;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#C-ABI-LAW");

/// The maximum number of bytes inspected for a static manifest, including NUL.
pub(crate) const MANIFEST_CAP: usize = 1024 * 1024;
pub(crate) const REQUIRED_SYMBOLS: [&str; 4] = [
    "vibe_ext_abi",
    "vibe_ext_manifest",
    "vibe_ext_invoke",
    "vibe_ext_free",
];

type AbiFn = unsafe extern "C" fn() -> u32;
type ManifestFn = unsafe extern "C" fn() -> *const c_char;
type InvokeFn = unsafe extern "C" fn(*const u8, usize, *mut *mut u8, *mut usize) -> i32;
type FreeFn = unsafe extern "C" fn(*mut u8, usize);

pub(crate) trait LibraryHandle: Send + Sync {
    fn abi(&self) -> u32;
    fn manifest_bytes(&self, path: &str) -> Result<Vec<u8>, NativeLoadError>;
    fn invoke(&self, request: &[u8], owner: Arc<dyn LibraryHandle>) -> CallResult;
}

pub(crate) trait LibraryOpener: Send + Sync {
    fn open(
        &self,
        canonical_path: &std::path::Path,
        display_path: &str,
    ) -> Result<Arc<dyn LibraryHandle>, NativeLoadError>;
}

pub(crate) struct SystemOpener;

impl LibraryOpener for SystemOpener {
    fn open(
        &self,
        canonical_path: &std::path::Path,
        display_path: &str,
    ) -> Result<Arc<dyn LibraryHandle>, NativeLoadError> {
        LoadedLibrary::open(canonical_path, display_path)
            .map(|library| Arc::new(library) as Arc<dyn LibraryHandle>)
    }
}

struct LoadedLibrary {
    _library: Library,
    abi: AbiFn,
    manifest: ManifestFn,
    invoke: InvokeFn,
    free: FreeFn,
}

impl LoadedLibrary {
    fn open(path: &std::path::Path, display_path: &str) -> Result<Self, NativeLoadError> {
        // SAFETY: Opening foreign code is this crate's explicit audit purpose.
        // The returned Library stays in the same object as every copied symbol.
        let library = unsafe { Library::new(path) }.map_err(|_| NativeLoadError::LibraryOpen {
            path: display_path.to_owned(),
        })?;
        let abi = load_symbol(
            &library,
            b"vibe_ext_abi\0",
            REQUIRED_SYMBOLS[0],
            display_path,
        )?;
        let manifest = load_symbol(
            &library,
            b"vibe_ext_manifest\0",
            REQUIRED_SYMBOLS[1],
            display_path,
        )?;
        let invoke = load_symbol(
            &library,
            b"vibe_ext_invoke\0",
            REQUIRED_SYMBOLS[2],
            display_path,
        )?;
        let free = load_symbol(
            &library,
            b"vibe_ext_free\0",
            REQUIRED_SYMBOLS[3],
            display_path,
        )?;
        Ok(Self {
            _library: library,
            abi,
            manifest,
            invoke,
            free,
        })
    }
}

fn load_symbol<T: Copy>(
    library: &Library,
    bytes: &[u8],
    name: &'static str,
    display_path: &str,
) -> Result<T, NativeLoadError> {
    // SAFETY: Each requested type exactly matches the frozen four-symbol ABI.
    // The function pointer is copied while `library` remains owned by the
    // enclosing LoadedLibrary.
    unsafe { library.get::<T>(bytes) }
        .map(|symbol| *symbol)
        .map_err(|_| NativeLoadError::MissingSymbol {
            path: display_path.to_owned(),
            symbol: name,
        })
}

impl LibraryHandle for LoadedLibrary {
    fn abi(&self) -> u32 {
        // SAFETY: The pointer was resolved with the exact ABI signature and its
        // owning library remains alive in `self`.
        unsafe { (self.abi)() }
    }

    fn manifest_bytes(&self, path: &str) -> Result<Vec<u8>, NativeLoadError> {
        // SAFETY: The pointer was resolved with the exact ABI signature.
        let pointer = unsafe { (self.manifest)() };
        if pointer.is_null() {
            return Err(NativeLoadError::ManifestPointerNull {
                path: path.to_owned(),
            });
        }
        let pointer = pointer.cast::<u8>();
        for len in 0..MANIFEST_CAP {
            // SAFETY: ABI 1 requires a readable static C string. The bounded
            // scan refuses a plugin that does not provide NUL within the cap.
            if unsafe { pointer.add(len).read() } == 0 {
                // SAFETY: The preceding scan established readable bytes through
                // `len`; copy immediately so no borrowed foreign memory escapes.
                return Ok(unsafe { std::slice::from_raw_parts(pointer, len) }.to_vec());
            }
        }
        Err(NativeLoadError::ManifestTooLarge {
            path: path.to_owned(),
            cap: MANIFEST_CAP,
        })
    }

    fn invoke(&self, request: &[u8], owner: Arc<dyn LibraryHandle>) -> CallResult {
        let mut pointer = std::ptr::null_mut();
        let mut len = 0usize;
        // SAFETY: The function pointer has the frozen ABI signature. Request
        // bytes remain borrowed for the call and both host slots are writable.
        let status = unsafe {
            (self.invoke)(
                request.as_ptr(),
                request.len(),
                &raw mut pointer,
                &raw mut len,
            )
        };
        let response = NonNull::new(pointer)
            .map(|pointer| PublishedResponse::native(pointer, len, self.free, owner));
        CallResult {
            status,
            response,
            len,
        }
    }
}

pub(crate) struct CallResult {
    pub(crate) status: i32,
    pub(crate) response: Option<PublishedResponse>,
    pub(crate) len: usize,
}

pub(crate) struct PublishedResponse {
    pointer: NonNull<u8>,
    len: usize,
    free: Option<FreeFn>,
    _owner: Option<Arc<dyn LibraryHandle>>,
    #[cfg(test)]
    _fake_bytes: Option<Box<[u8]>>,
    #[cfg(test)]
    fake_free_count: Option<Arc<std::sync::atomic::AtomicUsize>>,
}

impl PublishedResponse {
    fn native(
        pointer: NonNull<u8>,
        len: usize,
        free: FreeFn,
        owner: Arc<dyn LibraryHandle>,
    ) -> Self {
        Self {
            pointer,
            len,
            free: Some(free),
            _owner: Some(owner),
            #[cfg(test)]
            _fake_bytes: None,
            #[cfg(test)]
            fake_free_count: None,
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        // SAFETY: Callers validate nonzero, fixed-cap and isize-safe length
        // before asking for bytes. ABI 1 assigns readability of this exact pair
        // to the host until the guard frees it.
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.len) }
    }

    #[cfg(test)]
    pub(crate) fn fake(
        bytes: Vec<u8>,
        reported_len: usize,
        free_count: Arc<std::sync::atomic::AtomicUsize>,
        owner: Arc<dyn LibraryHandle>,
    ) -> Self {
        let mut bytes = bytes.into_boxed_slice();
        let pointer = if bytes.is_empty() {
            NonNull::dangling()
        } else {
            NonNull::from(&mut bytes[0])
        };
        Self {
            pointer,
            len: reported_len,
            free: None,
            _owner: Some(owner),
            _fake_bytes: Some(bytes),
            fake_free_count: Some(free_count),
        }
    }
}

impl Drop for PublishedResponse {
    fn drop(&mut self) {
        if let Some(free) = self.free.take() {
            // SAFETY: This guard is created exactly once for a published ABI
            // pair and Drop runs once while the strong library owner is alive.
            unsafe { free(self.pointer.as_ptr(), self.len) };
        }
        #[cfg(test)]
        if let Some(count) = &self.fake_free_count {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
