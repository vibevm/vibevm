use super::*;

#[test]
fn admitted_compiler_handle_refuses_before_invoke_and_reuses_the_exact_handle() {
    let (_directory, path) = fake_file();
    let (loader, library, _opener) = loader_for(
        manifest(&[("compiler", "compile:pass", Some(1))]),
        [FakeCall::published(0, b"reply".to_vec())],
    );
    let compiler = loader
        .admit_compile(&path, "compiler", CompilePoint::Pass)
        .unwrap();
    assert_eq!(compiler.extension_id(), "compiler");
    assert_eq!(compiler.point(), CompilePoint::Pass);
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    assert_eq!(compiler.invoke(b"request").unwrap(), b"reply");
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 1);

    let (loader, library, _) = loader_for(
        manifest(&[("other", "compile:pass", Some(1))]),
        [FakeCall::published(0, b"unused".to_vec())],
    );
    assert!(matches!(
        loader.admit_compile(&path, "compiler", CompilePoint::Pass),
        Err(NativeLoadError::MissingExtensionId { .. })
    ));
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
}

#[test]
fn admitted_compiler_wrong_abi_stops_before_manifest_or_invoke() {
    let (_directory, path) = fake_file();
    let library = Arc::new(FakeLibrary {
        abi: 9,
        manifest: manifest(&[("compiler", "compile:pass", Some(1))]),
        calls: Mutex::new(VecDeque::from([FakeCall::published(0, b"unused".to_vec())])),
        invoke_count: AtomicUsize::new(0),
        free_count: Arc::new(AtomicUsize::new(0)),
        requests: Mutex::new(Vec::new()),
    });
    let opener = Arc::new(FakeOpener {
        library: Arc::clone(&library),
        open_count: AtomicUsize::new(0),
    });
    let loader = NativeLoader::with_opener(opener as Arc<dyn ffi::LibraryOpener>);
    assert!(matches!(
        loader.admit_compile(&path, "compiler", CompilePoint::Pass),
        Err(NativeLoadError::AbiMismatch { actual: 9, .. })
    ));
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
}
