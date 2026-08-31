use std::collections::BTreeMap;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use vibe_core::manifest::ExtensionHandler;

use super::lowering_worlds::{Declared, collected_host};
use super::native_identity::NativeHandlerIdentity;
use super::native_identity::{
    CompilerNativeImplementationDigestError, compiler_native_implementation_digest,
};

fn native(crate_dir: Option<&str>, prebuilt: Option<Vec<(&str, &str)>>) -> ExtensionHandler {
    ExtensionHandler::Native {
        crate_dir: crate_dir.map(PathBuf::from),
        prebuilt: prebuilt.map(|pairs| {
            pairs
                .into_iter()
                .map(|(platform, path)| (platform.to_string(), PathBuf::from(path)))
                .collect()
        }),
    }
}

fn digest(handler: ExtensionHandler) -> [u8; 32] {
    let registry = collected_host(vec![Declared {
        id: "native",
        point: "compile:source",
        handler,
        config: None,
        applies_to: None,
        compiler_internals: None,
    }]);
    *compiler_native_implementation_digest(registry.enabled_compile_rows()[0])
        .expect("the native row has canonical UTF-8 identity")
        .as_bytes()
}

fn field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn longhand() -> [u8; 32] {
    let mut hasher = Sha256::new();
    field(&mut hasher, b"vibe-transform-implementation-v1\0epoch=1\0");
    hasher.update([1]);
    hasher.update(1_u32.to_le_bytes());
    hasher.update(1_u32.to_le_bytes());
    hasher.update([1]);
    field(&mut hasher, b"crates/native");
    hasher.update([1]);
    hasher.update(2_u64.to_le_bytes());
    field(&mut hasher, b"linux-x86_64");
    field(&mut hasher, b"bin/native.so");
    field(&mut hasher, b"windows-x86_64");
    field(&mut hasher, b"bin/native.dll");
    hasher.finalize().into()
}

#[test]
fn native_digest_matches_the_longhand_epoch_one_frame() {
    let actual = digest(native(
        Some("crates/native"),
        Some(vec![
            ("windows-x86_64", "bin/native.dll"),
            ("linux-x86_64", "bin/native.so"),
        ]),
    ));
    assert_eq!(actual, longhand());
    assert_eq!(
        actual,
        [
            0x7b, 0x37, 0xb4, 0xa7, 0x58, 0x32, 0x4f, 0xd3, 0x51, 0xec, 0x8b, 0x09, 0x39, 0xb4,
            0xe2, 0x8b, 0x73, 0x0f, 0xe9, 0x3c, 0xc7, 0x76, 0xf4, 0x59, 0xec, 0x60, 0xd7, 0xb8,
            0xd1, 0x99, 0xf2, 0xd9,
        ]
    );
}

#[test]
fn every_authored_native_presence_and_member_moves_identity_independently() {
    let absent = *NativeHandlerIdentity::candidate(None, None)
        .digest()
        .as_bytes();
    let crate_present = digest(native(Some("crates/native"), None));
    let crate_moved = digest(native(Some("crates/other"), None));
    let prebuilt_empty = digest(native(None, Some(vec![])));
    let prebuilt_one = digest(native(
        None,
        Some(vec![("windows-x86_64", "bin/native.dll")]),
    ));
    let prebuilt_two = digest(native(
        None,
        Some(vec![
            ("windows-x86_64", "bin/native.dll"),
            ("linux-x86_64", "bin/native.so"),
        ]),
    ));
    let platform_moved = digest(native(None, Some(vec![("linux-x86_64", "bin/native.dll")])));
    let path_moved = digest(native(
        None,
        Some(vec![("windows-x86_64", "bin/other.dll")]),
    ));
    let values = [
        absent,
        crate_present,
        crate_moved,
        prebuilt_empty,
        prebuilt_one,
        prebuilt_two,
        platform_moved,
        path_moved,
    ];
    for (index, value) in values.iter().enumerate() {
        assert!(
            values[..index].iter().all(|previous| previous != value),
            "case {index} has its own identity"
        );
    }
}

#[test]
fn prebuilt_input_order_canonicalizes_to_byte_key_order() {
    let first = digest(native(
        None,
        Some(vec![
            ("windows-x86_64", "bin/native.dll"),
            ("linux-x86_64", "bin/native.so"),
        ]),
    ));
    let second = digest(ExtensionHandler::Native {
        crate_dir: None,
        prebuilt: Some(BTreeMap::from([
            ("linux-x86_64".to_string(), PathBuf::from("bin/native.so")),
            (
                "windows-x86_64".to_string(),
                PathBuf::from("bin/native.dll"),
            ),
        ])),
    });
    assert_eq!(first, second);
}

#[cfg(unix)]
fn invalid_os_string() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(vec![0xff])
}

#[cfg(windows)]
fn invalid_os_string() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(&[0xd800])
}

#[test]
fn a_non_utf8_crate_dir_refuses_instead_of_colliding_with_lossy_replacement() {
    let invalid = ExtensionHandler::Native {
        crate_dir: Some(PathBuf::from(invalid_os_string())),
        prebuilt: None,
    };
    assert_eq!(
        NativeHandlerIdentity::from_handler(&invalid),
        Err(CompilerNativeImplementationDigestError::NonUtf8CrateDir)
    );

    let replacement = ExtensionHandler::Native {
        crate_dir: Some(PathBuf::from("�")),
        prebuilt: None,
    };
    assert!(
        NativeHandlerIdentity::from_handler(&replacement)
            .unwrap()
            .is_some(),
        "a real replacement character is a lawful identity, never the alias of invalid OS bytes"
    );
}

#[test]
fn a_non_utf8_prebuilt_path_refuses_with_bounded_platform_attribution() {
    let invalid = ExtensionHandler::Native {
        crate_dir: None,
        prebuilt: Some(BTreeMap::from([(
            "windows-x86_64".to_string(),
            PathBuf::from(invalid_os_string()),
        )])),
    };
    assert_eq!(
        NativeHandlerIdentity::from_handler(&invalid),
        Err(
            CompilerNativeImplementationDigestError::NonUtf8PrebuiltPath {
                platform: "windows-x86_64".to_string(),
            }
        )
    );
}
