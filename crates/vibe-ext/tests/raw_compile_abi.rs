use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::CStr;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_ext::{
    CompileReply, CompileReplyFail, CompileReplyOk, CompileReplySkip, CompileRequest, Ir, Manifest,
    ManifestExtension,
};

const VALID_REQUEST: &str =
    include_str!("../../../formats/corpora/native/e1/compile_request.valid.json");
const DOCUMENT_IR: &str =
    include_str!("../../../formats/corpora/compiler_ir/e1/valid/document_document.json");

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
static MOVE_BEFORE: AtomicUsize = AtomicUsize::new(0);
static MOVE_AFTER: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn payload_allocation(payload: &Ir) -> usize {
    match payload {
        Ir::SourceDocument(value) => value.as_ref() as *const _ as usize,
        Ir::DocumentDocument(value) => value.as_ref() as *const _ as usize,
        Ir::DocumentsArtifact(value) => value.as_ref() as *const _ as usize,
        Ir::ClosureArtifact(value) => value.as_ref() as *const _ as usize,
        Ir::LaneArtifact(value) => value.as_ref() as *const _ as usize,
        Ir::EmittedArtifact(value) => value.as_ref() as *const _ as usize,
    }
}

fn handle(request: CompileRequest) -> CompileReply {
    HANDLER_CALLS.fetch_add(1, Ordering::SeqCst);
    match request.execution.id.as_str() {
        "compiler-skip" => CompileReply::Skip(Box::new(CompileReplySkip {
            envelope: 1,
            message: Some("skip".to_owned()),
        })),
        "compiler-fail" => CompileReply::Fail(Box::new(CompileReplyFail {
            envelope: 1,
            message: Some("fail".to_owned()),
        })),
        "compiler-panic" => panic!("compiler handler panic"),
        "bad-reply-envelope" => CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 2,
            payload: request.payload,
            message: None,
        })),
        "changed-shape" => CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 1,
            payload: serde_json::from_str(DOCUMENT_IR).expect("document IR corpus decodes"),
            message: None,
        })),
        _ => {
            let payload = request.payload;
            MOVE_BEFORE.store(payload_allocation(&payload), Ordering::SeqCst);
            let reply = CompileReply::Ok(Box::new(CompileReplyOk {
                envelope: 1,
                payload,
                message: Some("ok".to_owned()),
            }));
            let CompileReply::Ok(value) = &reply else {
                unreachable!()
            };
            MOVE_AFTER.store(payload_allocation(&value.payload), Ordering::SeqCst);
            reply
        }
    }
}

vibe_ext::vibe_compile_extension!(
    manifest = Manifest {
        extensions: vec![ManifestExtension {
            id: "compiler-fixture".to_owned(),
            point: "compile:pass".to_owned(),
            ir_schema: Some(1),
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

fn request_value(id: &str) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_json::from_str(VALID_REQUEST).expect("canonical request corpus decodes");
    value["execution"]["id"] = id.into();
    value
}

fn request_bytes(id: &str) -> Vec<u8> {
    serde_json::to_vec(&request_value(id)).expect("request JSON")
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

fn decode_and_free(response: *mut u8, response_len: usize) -> serde_json::Value {
    // SAFETY: Successful invoke publishes this exact readable slice.
    let bytes = unsafe { std::slice::from_raw_parts(response, response_len) };
    let value = serde_json::from_slice(bytes).expect("compiler reply JSON");
    vibe_ext_free(response, response_len);
    value
}

fn assert_refused(request: &[u8]) {
    let (status, response, response_len) = invoke_bytes(request);
    assert_ne!(status, 0);
    assert!(response.is_null());
    assert_eq!(response_len, 0);
}

#[test]
fn exact_symbols_abi_and_stable_compiler_manifest() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    // SAFETY: Declarations bind the exact macro-emitted ABI symbols.
    assert_eq!(unsafe { linked_vibe_ext_abi() }, 1);
    assert_eq!(vibe_ext_abi(), 1);

    let first = vibe_ext_manifest();
    let second = vibe_ext_manifest();
    assert_eq!(first, second);
    assert!(!first.is_null());
    // SAFETY: The manifest ABI returns a stable NUL-terminated CString.
    let manifest: Manifest = serde_json::from_slice(unsafe { CStr::from_ptr(first) }.to_bytes())
        .expect("generated compiler manifest");
    assert_eq!(manifest.extensions.len(), 1);
    assert!(manifest.extensions[0].point.starts_with("compile:"));
    assert_eq!(manifest.extensions[0].ir_schema, Some(1));

    // SAFETY: The linked manifest symbol has the same stable-pointer contract.
    let linked = unsafe { linked_vibe_ext_manifest() };
    assert_eq!(linked, first);
}

#[test]
fn valid_request_moves_ir_and_uses_exact_once_boxed_slice_free() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    MOVE_BEFORE.store(0, Ordering::SeqCst);
    MOVE_AFTER.store(0, Ordering::SeqCst);
    let request = request_bytes("compiler-ok");
    let mut response = ptr::null_mut();
    let mut response_len = 0;
    // SAFETY: Request and output slots remain valid for the call.
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
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 1);
    assert_ne!(MOVE_BEFORE.load(Ordering::SeqCst), 0);
    assert_eq!(
        MOVE_BEFORE.load(Ordering::SeqCst),
        MOVE_AFTER.load(Ordering::SeqCst),
        "the handler moves the same boxed IR allocation into its reply"
    );
    // SAFETY: The published response is readable for its exact length.
    let reply: CompileReply =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(response, response_len) })
            .expect("typed compiler reply");
    assert!(matches!(reply, CompileReply::Ok(_)));

    let frees_before = TRACKED_FREES.load(Ordering::SeqCst);
    TRACKED_RESPONSE.store(response as usize, Ordering::SeqCst);
    // SAFETY: The exact pointer/length pair is returned once to the linked free.
    unsafe { linked_vibe_ext_free(response, response_len) };
    assert_eq!(TRACKED_RESPONSE.load(Ordering::SeqCst), 0);
    assert_eq!(TRACKED_FREES.load(Ordering::SeqCst), frees_before + 1);
}

#[test]
fn skip_and_fail_serialize_without_payload() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    for (id, status_name) in [("compiler-skip", "skip"), ("compiler-fail", "fail")] {
        let (status, response, response_len) = invoke_bytes(&request_bytes(id));
        assert_eq!(status, 0);
        let reply = decode_and_free(response, response_len);
        assert_eq!(reply["status"], status_name);
        assert!(reply.get("payload").is_none());
    }
}

#[test]
fn structurally_or_behaviorally_bad_requests_never_call_handler() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    HANDLER_CALLS.store(0, Ordering::SeqCst);
    assert_refused(b"not-json");

    let compact =
        serde_json::to_string(&request_value("duplicate")).expect("compact canonical request");
    let duplicate = compact.replacen(
        r#""shape":"source-document""#,
        r#""shape":"source-document","shape":"source-document""#,
        1,
    );
    assert_ne!(duplicate, compact);
    assert_refused(duplicate.as_bytes());

    let mut wrong_envelope = request_value("wrong-envelope");
    wrong_envelope["envelope"] = 2.into();
    assert_refused(&serde_json::to_vec(&wrong_envelope).unwrap());

    let mut wrong_schema = request_value("wrong-schema");
    wrong_schema["payload"]["ir_schema"] = 2.into();
    assert_refused(&serde_json::to_vec(&wrong_schema).unwrap());

    let mut unsupported = request_value("unsupported");
    unsupported["point"] = "vendor:future".into();
    assert_refused(&serde_json::to_vec(&unsupported).unwrap());

    let mut mismatch = request_value("mismatch");
    mismatch["point"] = "compile:lane".into();
    assert_refused(&serde_json::to_vec(&mismatch).unwrap());
    assert_eq!(HANDLER_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn invalid_handler_replies_are_never_published() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    assert_refused(&request_bytes("bad-reply-envelope"));
    assert_refused(&request_bytes("changed-shape"));
}

#[test]
fn compiler_handler_panic_is_contained_and_later_call_succeeds() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    assert_refused(&request_bytes("compiler-panic"));
    let (status, response, response_len) = invoke_bytes(&request_bytes("compiler-after"));
    assert_eq!(status, 0);
    let reply = decode_and_free(response, response_len);
    assert_eq!(reply["status"], "ok");
}

#[test]
fn null_request_output_slots_and_null_free_match_lifecycle_contract() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let request = request_bytes("compiler-ok");

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

#[test]
fn both_public_macros_use_the_one_emitter_and_compiler_dispatch_never_clones_ir() {
    let source = include_str!("../src/lib.rs");
    assert_eq!(source.matches("$crate::__vibe_ext_emit_abi!(").count(), 2);
    assert_eq!(source.matches("fn initialize_outputs(").count(), 1);
    assert_eq!(
        source.matches("fn free(ptr: *mut u8, len: usize)").count(),
        1
    );
    assert_eq!(source.matches("catch_unwind(AssertUnwindSafe(").count(), 1);

    let compiler = source
        .split_once("macro_rules! vibe_compile_extension")
        .expect("compiler macro exists")
        .1;
    assert!(compiler.contains("validate_request(&request)"));
    assert!(compiler.contains("validate_reply_for_shape(request_shape, &reply)"));
    assert!(!compiler.contains("request.clone()"));
    assert!(!compiler.contains("payload.clone()"));
}
