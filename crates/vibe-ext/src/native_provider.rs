//! Safe build/package mechanism-provider author boundary.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#R8-NATIVE-BUILD-PACKAGE-OPEN");

use vibe_wire::behaviour::{native_build, native_mechanism, native_package};
use vibe_wire::generated::native::e1::{
    build_reply::BuildReply, build_request::BuildRequest, mechanism_manifest::MechanismManifest,
    package_reply::PackageReply, package_request::PackageRequest,
};
use vibe_wire::generated::shared::NativeMechanismRole;

pub fn validate_build_manifest(manifest: &MechanismManifest) -> bool {
    manifest_role(manifest, NativeMechanismRole::Build)
}

pub fn validate_package_manifest(manifest: &MechanismManifest) -> bool {
    manifest_role(manifest, NativeMechanismRole::Package)
}

fn manifest_role(manifest: &MechanismManifest, expected: NativeMechanismRole) -> bool {
    native_mechanism::validate_manifest(manifest).is_ok()
        && !manifest.mechanisms.is_empty()
        && manifest
            .mechanisms
            .iter()
            .all(|descriptor| descriptor.role == expected)
}

pub fn admit_build_request(manifest: &MechanismManifest, request: &BuildRequest) -> bool {
    let (provider, mechanism, target) = build_identity(request);
    native_build::validate_request(
        request,
        provider,
        mechanism,
        target,
        retained_build(request),
    )
    .is_ok()
        && descriptor_matches(manifest, provider, mechanism, NativeMechanismRole::Build)
}

pub fn admit_package_request(manifest: &MechanismManifest, request: &PackageRequest) -> bool {
    let (provider, mechanism, target) = package_identity(request);
    native_package::validate_request(
        request,
        provider,
        mechanism,
        target,
        retained_package(request),
    )
    .is_ok()
        && descriptor_matches(manifest, provider, mechanism, NativeMechanismRole::Package)
}

pub fn validate_build_exchange(request: &BuildRequest, reply: &BuildReply) -> bool {
    let (provider, mechanism, target) = build_identity(request);
    native_build::validate_exchange(
        request,
        provider,
        mechanism,
        target,
        retained_build(request),
        reply,
    )
    .is_ok()
}

pub fn validate_package_exchange(request: &PackageRequest, reply: &PackageReply) -> bool {
    let (provider, mechanism, target) = package_identity(request);
    native_package::validate_exchange(
        request,
        provider,
        mechanism,
        target,
        retained_package(request),
        reply,
    )
    .is_ok()
}

fn descriptor_matches(
    manifest: &MechanismManifest,
    provider: &str,
    mechanism: &str,
    role: NativeMechanismRole,
) -> bool {
    let Some((_, descriptor_id)) = provider.rsplit_once('#') else {
        return false;
    };
    let prefix = match role {
        NativeMechanismRole::Build => "build:",
        NativeMechanismRole::Package => "package:",
        NativeMechanismRole::Deploy | NativeMechanismRole::Acquire => return false,
    };
    let Some(name) = mechanism.strip_prefix(prefix) else {
        return false;
    };
    manifest.mechanisms.iter().any(|descriptor| {
        descriptor.id == descriptor_id && descriptor.role == role && descriptor.name == name
    })
}

fn build_identity(request: &BuildRequest) -> (&str, &str, &str) {
    let identity = match request {
        BuildRequest::Plan(value) => &value.identity,
        BuildRequest::Fingerprint(value) => &value.identity,
        BuildRequest::Apply(value) => &value.identity,
        BuildRequest::Verify(value) => &value.identity,
    };
    (&identity.provider, &identity.mechanism, &identity.target)
}

fn package_identity(request: &PackageRequest) -> (&str, &str, &str) {
    let identity = match request {
        PackageRequest::Plan(value) => &value.identity,
        PackageRequest::Fingerprint(value) => &value.identity,
        PackageRequest::Apply(value) => &value.identity,
        PackageRequest::Verify(value) => &value.identity,
    };
    (&identity.provider, &identity.mechanism, &identity.target)
}

fn retained_build(request: &BuildRequest) -> native_build::RetainedBuild<'_> {
    match request {
        BuildRequest::Plan(_) => native_build::RetainedBuild::default(),
        BuildRequest::Fingerprint(value) => native_build::RetainedBuild {
            target: Some(&value.target),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            ..native_build::RetainedBuild::default()
        },
        BuildRequest::Apply(value) => native_build::RetainedBuild {
            target: Some(&value.target),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            ..native_build::RetainedBuild::default()
        },
        BuildRequest::Verify(value) => native_build::RetainedBuild {
            target: Some(&value.target),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            staging: Some(&value.staging),
            staged: Some(&value.staged),
        },
    }
}

fn retained_package(request: &PackageRequest) -> native_package::RetainedPackage<'_> {
    match request {
        PackageRequest::Plan(_) => native_package::RetainedPackage::default(),
        PackageRequest::Fingerprint(value) => native_package::RetainedPackage {
            target: Some(&value.target),
            inputs: Some(&value.inputs),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            ..native_package::RetainedPackage::default()
        },
        PackageRequest::Apply(value) => native_package::RetainedPackage {
            target: Some(&value.target),
            inputs: Some(&value.inputs),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            ..native_package::RetainedPackage::default()
        },
        PackageRequest::Verify(value) => native_package::RetainedPackage {
            target: Some(&value.target),
            inputs: Some(&value.inputs),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            staging: Some(&value.staging),
            staged: Some(&value.staged),
        },
    }
}

/// Export one build-mechanism provider through the shared four-symbol ABI 1.
#[macro_export]
macro_rules! vibe_build_provider {
    (manifest = $manifest:expr, handler = $handler:path $(,)?) => {
        #[doc(hidden)]
        fn __vibe_ext_mechanism_manifest() -> &'static $crate::MechanismManifest {
            static MANIFEST: std::sync::OnceLock<$crate::MechanismManifest> =
                std::sync::OnceLock::new();
            MANIFEST.get_or_init(|| $manifest)
        }

        #[doc(hidden)]
        fn __vibe_ext_manifest_value() -> $crate::MechanismManifest {
            __vibe_ext_mechanism_manifest().clone()
        }

        #[doc(hidden)]
        fn __vibe_ext_handle(request: $crate::BuildRequest) -> $crate::BuildReply {
            $handler(request)
        }

        #[doc(hidden)]
        fn __vibe_ext_dispatch(request: &[u8]) -> Option<Vec<u8>> {
            let request: $crate::BuildRequest = $crate::__serde_json::from_slice(request).ok()?;
            let manifest = __vibe_ext_mechanism_manifest();
            static MANIFEST_ADMITTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if !*MANIFEST_ADMITTED
                .get_or_init(|| $crate::native_provider::validate_build_manifest(manifest))
                || !$crate::native_provider::admit_build_request(manifest, &request)
            {
                return None;
            }
            let retained = request.clone();
            let reply = __vibe_ext_handle(request);
            if !$crate::native_provider::validate_build_exchange(&retained, &reply) {
                return None;
            }
            let encoded = $crate::__serde_json::to_vec(&reply).ok()?;
            (encoded.len() <= $crate::__native_build::REPLY_CAP_BYTES).then_some(encoded)
        }

        $crate::__vibe_ext_emit_abi!(
            panic_message = "vibe_build_provider! requires panic = \"unwind\"; remove panic = \"abort\" from the provider's active Cargo profile",
        );
    };
}

/// Export one package-mechanism provider through the shared four-symbol ABI 1.
#[macro_export]
macro_rules! vibe_package_provider {
    (manifest = $manifest:expr, handler = $handler:path $(,)?) => {
        #[doc(hidden)]
        fn __vibe_ext_mechanism_manifest() -> &'static $crate::MechanismManifest {
            static MANIFEST: std::sync::OnceLock<$crate::MechanismManifest> =
                std::sync::OnceLock::new();
            MANIFEST.get_or_init(|| $manifest)
        }

        #[doc(hidden)]
        fn __vibe_ext_manifest_value() -> $crate::MechanismManifest {
            __vibe_ext_mechanism_manifest().clone()
        }

        #[doc(hidden)]
        fn __vibe_ext_handle(request: $crate::PackageRequest) -> $crate::PackageReply {
            $handler(request)
        }

        #[doc(hidden)]
        fn __vibe_ext_dispatch(request: &[u8]) -> Option<Vec<u8>> {
            let request: $crate::PackageRequest = $crate::__serde_json::from_slice(request).ok()?;
            let manifest = __vibe_ext_mechanism_manifest();
            static MANIFEST_ADMITTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if !*MANIFEST_ADMITTED
                .get_or_init(|| $crate::native_provider::validate_package_manifest(manifest))
                || !$crate::native_provider::admit_package_request(manifest, &request)
            {
                return None;
            }
            let retained = request.clone();
            let reply = __vibe_ext_handle(request);
            if !$crate::native_provider::validate_package_exchange(&retained, &reply) {
                return None;
            }
            let encoded = $crate::__serde_json::to_vec(&reply).ok()?;
            (encoded.len() <= $crate::__native_package::REPLY_CAP_BYTES).then_some(encoded)
        }

        $crate::__vibe_ext_emit_abi!(
            panic_message = "vibe_package_provider! requires panic = \"unwind\"; remove panic = \"abort\" from the provider's active Cargo profile",
        );
    };
}
