use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_ext::{
    Context, Execution, Io, Manifest, ManifestExtension, Project, Reply, ReplyArtifact,
    ReplyStatus, Run, RunAgentMode, World,
};

struct TrackingAllocator;

static TRACKED_RESPONSE: AtomicUsize = AtomicUsize::new(0);
static TRACKED_FREES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegates the allocation request unchanged to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACKED_RESPONSE
            .compare_exchange(ptr as usize, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            TRACKED_FREES.fetch_add(1, Ordering::SeqCst);
        }
        // SAFETY: Delegates the matching deallocation unchanged to System.
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

static HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn handle(context: Context) -> Reply {
    HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
    if context.point == "phase:panic" {
        panic!("fixture handler panic");
    }
    Reply {
        artifacts: vec![ReplyArtifact {
            id: context.execution.id,
            kind: "file".to_owned(),
            path: "target/result.txt".to_owned(),
        }],
        envelope: 1,
        status: ReplyStatus::Ok,
        message: Some(context.point),
    }
}

vibe_ext::vibe_extension!(
    manifest = Manifest {
        extensions: vec![ManifestExtension {
            id: "fixture".to_owned(),
            point: "phase:test".to_owned(),
            ir_schema: None,
        }],
    },
    handler = handle,
);

unsafe extern "C" {
    #[link_name = "vibe_ext_abi"]
    fn linked_vibe_ext_abi() -> u32;

    #[link_name = "vibe_ext_manifest"]
    fn linked_vibe_ext_manifest() -> *const std::ffi::c_char;

    #[link_name = "vibe_ext_invoke"]
    fn linked_vibe_ext_invoke(
        request_ptr: *const u8,
        request_len: usize,
        response_ptr: *mut *mut u8,
        response_len: *mut usize,
    ) -> i32;

    #[link_name = "vibe_ext_free"]
    fn linked_vibe_ext_free(ptr: *mut u8, len: usize);
}

fn context(point: &str, envelope: u32) -> Context {
    Context {
        artifacts: Vec::new(),
        envelope,
        execution: Execution {
            config: BTreeMap::new(),
            id: "fixture".to_owned(),
            package: "org.example/fixture".to_owned(),
        },
        io: Io {
            scratch: ".vibe/scratch".to_owned(),
        },
        point: point.to_owned(),
        project: Project {
            kind: "flow".to_owned(),
            manifest: "vibe.toml".to_owned(),
            name: "project".to_owned(),
            root: ".".to_owned(),
            spec_roots: vec!["vibevm/vibespecs".to_owned()],
            version: "1.0.0".to_owned(),
        },
        run: Run {
            agent_mode: RunAgentMode::Cli,
            assume_yes: false,
            chain: vec!["validate".to_owned(), "test".to_owned()],
            force: false,
            offline: true,
            phase: "test".to_owned(),
            requested: "test".to_owned(),
        },
        world: World {
            deps_root: "vibevm/vibedeps".to_owned(),
            lockfile: "vibe.lock".to_owned(),
            packages: Vec::new(),
        },
        slot_target: None,
    }
}

fn invoke_bytes(request: &[u8]) -> (i32, *mut u8, usize) {
    let mut response = ptr::dangling_mut::<u8>();
    let mut response_len = usize::MAX;
    let status = vibe_ext_invoke(
        request.as_ptr(),
        request.len(),
        &mut response,
        &mut response_len,
    );
    (status, response, response_len)
}

#[test]
fn exact_external_c_symbols_link_and_are_callable() {
    let _guard = TEST_LOCK.lock().expect("test lock");

    // SAFETY: These declarations bind the exact four macro-emitted C symbols
    // and each call follows its ABI contract.
    assert_eq!(unsafe { linked_vibe_ext_abi() }, 1);
    // SAFETY: The manifest symbol returns a stable NUL-terminated CString.
    let manifest_ptr = unsafe { linked_vibe_ext_manifest() };
    assert!(!manifest_ptr.is_null());
    // SAFETY: The pointer came from the linked manifest symbol above.
    let manifest: Manifest =
        serde_json::from_slice(unsafe { CStr::from_ptr(manifest_ptr) }.to_bytes())
            .expect("linked generated Manifest JSON");
    assert_eq!(manifest.extensions[0].id, "fixture");

    let request = serde_json::to_vec(&context("phase:test", 1)).expect("context JSON");
    let mut response = ptr::null_mut();
    let mut response_len = 0;
    // SAFETY: Request bytes live through the call and both output slots are
    // valid writable storage.
    let status = unsafe {
        linked_vibe_ext_invoke(
            request.as_ptr(),
            request.len(),
            &mut response,
            &mut response_len,
        )
    };
    assert_eq!(status, 0);
    assert!(!response.is_null());
    assert_ne!(response_len, 0);
    // SAFETY: Successful linked invoke published this exact readable slice.
    let reply: Reply =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(response, response_len) })
            .expect("linked generated Reply JSON");
    assert_eq!(reply.status, ReplyStatus::Ok);
    // SAFETY: The pointer/length pair is returned exactly once to its linked
    // free symbol.
    unsafe { linked_vibe_ext_free(response, response_len) };
}

#[test]
fn abi_and_manifest_are_stable_and_generated() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    assert_eq!(vibe_ext_abi(), 1);

    let first = vibe_ext_manifest();
    let second = vibe_ext_manifest();
    assert!(!first.is_null());
    assert_eq!(first, second);
    // SAFETY: The ABI promises a stable NUL-terminated CString pointer.
    let bytes = unsafe { CStr::from_ptr(first) }.to_bytes();
    let manifest: Manifest = serde_json::from_slice(bytes).expect("generated Manifest JSON");
    assert_eq!(manifest.extensions.len(), 1);
    assert_eq!(manifest.extensions[0].id, "fixture");
}

#[test]
fn valid_invoke_round_trips_reply_and_exact_free() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    let request = serde_json::to_vec(&context("phase:test", 1)).expect("context JSON");
    let (status, response, response_len) = invoke_bytes(&request);
    assert_eq!(status, 0);
    assert!(!response.is_null());
    assert_ne!(response_len, 0);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);

    // SAFETY: Successful invoke published a readable exact-length response.
    let response_bytes = unsafe { std::slice::from_raw_parts(response, response_len) };
    let reply: Reply = serde_json::from_slice(response_bytes).expect("generated Reply JSON");
    assert_eq!(reply.envelope, 1);
    assert_eq!(reply.status, ReplyStatus::Ok);
    assert_eq!(reply.artifacts[0].id, "fixture");

    let frees_before = TRACKED_FREES.load(Ordering::SeqCst);
    TRACKED_RESPONSE.store(response as usize, Ordering::SeqCst);
    vibe_ext_free(response, response_len);
    assert_eq!(TRACKED_RESPONSE.load(Ordering::SeqCst), 0);
    assert_eq!(TRACKED_FREES.load(Ordering::SeqCst), frees_before + 1);
}

#[test]
fn malformed_and_wrong_envelope_never_call_handler() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    HANDLER_CALLS.store(0, Ordering::SeqCst);

    let (status, response, response_len) = invoke_bytes(b"not-json");
    assert_ne!(status, 0);
    assert!(response.is_null());
    assert_eq!(response_len, 0);

    let wrong = serde_json::to_vec(&context("phase:test", 2)).expect("context JSON");
    let (status, response, response_len) = invoke_bytes(&wrong);
    assert_ne!(status, 0);
    assert!(response.is_null());
    assert_eq!(response_len, 0);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn handler_panic_is_contained_and_later_call_succeeds() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    HANDLER_CALLS.store(0, Ordering::SeqCst);

    let panicking = serde_json::to_vec(&context("phase:panic", 1)).expect("context JSON");
    let (status, response, response_len) = invoke_bytes(&panicking);
    assert_ne!(status, 0);
    assert!(response.is_null());
    assert_eq!(response_len, 0);

    let valid = serde_json::to_vec(&context("phase:test", 1)).expect("context JSON");
    let (status, response, response_len) = invoke_bytes(&valid);
    assert_eq!(status, 0);
    assert!(!response.is_null());
    assert_ne!(response_len, 0);
    vibe_ext_free(response, response_len);
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn invalid_pointers_refuse_and_null_free_is_noop() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let request = serde_json::to_vec(&context("phase:test", 1)).expect("context JSON");

    let mut len = usize::MAX;
    let status = vibe_ext_invoke(request.as_ptr(), request.len(), ptr::null_mut(), &mut len);
    assert_ne!(status, 0);
    assert_eq!(len, 0);

    let mut response = ptr::dangling_mut::<u8>();
    let status = vibe_ext_invoke(
        request.as_ptr(),
        request.len(),
        &mut response,
        ptr::null_mut(),
    );
    assert_ne!(status, 0);
    assert!(response.is_null());

    let mut response = ptr::dangling_mut::<u8>();
    let mut len = usize::MAX;
    let status = vibe_ext_invoke(ptr::null(), 1, &mut response, &mut len);
    assert_ne!(status, 0);
    assert!(response.is_null());
    assert_eq!(len, 0);

    let frees_before = TRACKED_FREES.load(Ordering::SeqCst);
    vibe_ext_free(ptr::null_mut(), 0);
    assert_eq!(TRACKED_FREES.load(Ordering::SeqCst), frees_before);
}
