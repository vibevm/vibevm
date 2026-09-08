//! Typed build/package calls on one already-admitted mechanism handle.

use vibe_core::manifest::MechanismRole;
use vibe_wire::behaviour::{native_build, native_package};
use vibe_wire::generated::native::e1::{
    build_reply::BuildReply, build_request::BuildRequest, package_reply::PackageReply,
    package_request::PackageRequest,
};
use vibe_wire::generated::shared::NativeDeployOperation;

use crate::{NativeLoadError, NativeMechanism, invoke_admitted, scalar_preview};

impl NativeMechanism {
    pub fn invoke_build(
        &self,
        target: &str,
        request: &BuildRequest,
        retained: native_build::RetainedBuild<'_>,
    ) -> Result<BuildReply, NativeLoadError> {
        self.require_role(MechanismRole::Build)?;
        let logical_key = self.logical_key.to_string();
        let operation =
            native_build::validate_request(request, &self.provider, &logical_key, target, retained)
                .map_err(|error| self.request_error(error.to_string()))?;
        self.require_operation(match operation {
            native_build::BuildOperation::Plan => NativeDeployOperation::Plan,
            native_build::BuildOperation::Fingerprint => NativeDeployOperation::Fingerprint,
            native_build::BuildOperation::Apply => NativeDeployOperation::Apply,
            native_build::BuildOperation::Verify => NativeDeployOperation::Verify,
        })?;
        let encoded = serde_json::to_vec(request).map_err(|error| {
            self.serialization_error(format!(
                "JSON at line {}, column {}",
                error.line(),
                error.column()
            ))
        })?;
        let response = invoke_admitted(&self.library, &encoded, &self.display_path)?;
        let reply = native_build::decode_reply(response.bytes())
            .map_err(|error| self.reply_error(error.to_string()))?;
        native_build::validate_exchange(
            request,
            &self.provider,
            &logical_key,
            target,
            retained,
            &reply,
        )
        .map_err(|error| self.reply_error(error.to_string()))?;
        Ok(reply)
    }

    pub fn invoke_package(
        &self,
        target: &str,
        request: &PackageRequest,
        retained: native_package::RetainedPackage<'_>,
    ) -> Result<PackageReply, NativeLoadError> {
        self.require_role(MechanismRole::Package)?;
        let logical_key = self.logical_key.to_string();
        let operation = native_package::validate_request(
            request,
            &self.provider,
            &logical_key,
            target,
            retained,
        )
        .map_err(|error| self.request_error(error.to_string()))?;
        self.require_operation(match operation {
            native_package::PackageOperation::Plan => NativeDeployOperation::Plan,
            native_package::PackageOperation::Fingerprint => NativeDeployOperation::Fingerprint,
            native_package::PackageOperation::Apply => NativeDeployOperation::Apply,
            native_package::PackageOperation::Verify => NativeDeployOperation::Verify,
        })?;
        let encoded = serde_json::to_vec(request).map_err(|error| {
            self.serialization_error(format!(
                "JSON at line {}, column {}",
                error.line(),
                error.column()
            ))
        })?;
        let response = invoke_admitted(&self.library, &encoded, &self.display_path)?;
        let reply = native_package::decode_reply(response.bytes())
            .map_err(|error| self.reply_error(error.to_string()))?;
        native_package::validate_exchange(
            request,
            &self.provider,
            &logical_key,
            target,
            retained,
            &reply,
        )
        .map_err(|error| self.reply_error(error.to_string()))?;
        Ok(reply)
    }

    fn require_role(&self, expected: MechanismRole) -> Result<(), NativeLoadError> {
        if self.expected_role == expected {
            Ok(())
        } else {
            Err(NativeLoadError::MechanismRoleMethod {
                path: self.display_path.clone(),
                actual: self.expected_role.to_string(),
                attempted: expected.to_string(),
            })
        }
    }

    fn require_operation(&self, operation: NativeDeployOperation) -> Result<(), NativeLoadError> {
        if self.descriptor.operations.contains(&operation) {
            Ok(())
        } else {
            Err(self.request_error(
                "requested operation is absent from the selected descriptor".to_owned(),
            ))
        }
    }

    fn request_error(&self, reason: String) -> NativeLoadError {
        NativeLoadError::MechanismRequestAdmission {
            path: self.display_path.clone(),
            id: scalar_preview(&self.descriptor.id),
            reason,
        }
    }

    fn serialization_error(&self, reason: String) -> NativeLoadError {
        NativeLoadError::MechanismRequestSerialization {
            path: self.display_path.clone(),
            id: scalar_preview(&self.descriptor.id),
            reason,
        }
    }

    fn reply_error(&self, reason: String) -> NativeLoadError {
        NativeLoadError::MechanismReplyAdmission {
            path: self.display_path.clone(),
            id: scalar_preview(&self.descriptor.id),
            reason,
        }
    }
}
