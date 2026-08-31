//! Canonical identity of one compiler-native implementation.
//!
//! This is the single digest implementation shared by plan lowering and the
//! later artifact-backed invoker adapter. Runtime resolution and materialized
//! paths never enter: only the authored handler declaration is framed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use std::collections::BTreeMap;
use std::ffi::OsStr;

use vibe_core::manifest::ExtensionHandler;
use vibe_extension_registry::ExtensionRegistryRow;

use crate::compiler::digest::StableDigest;

use super::plan_digest::IMPLEMENTATION_DIGEST_DOMAIN;

const TAG_IMPLEMENTATION_NATIVE: u8 = 1;
const NATIVE_ABI_EPOCH: u32 = 1;
const COMPILER_IR_SCHEMA: u32 = 1;

/// The opaque, comparable identity of one authored compiler-native handler.
///
/// The bytes stay private: callers may carry and compare this value but may
/// neither author a digest nor use it as a plan-construction surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerNativeImplementationDigest([u8; 32]);

impl CompilerNativeImplementationDigest {
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Why one exact registry row cannot name a compiler-native implementation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompilerNativeImplementationDigestError {
    #[error("the extension row does not declare a native handler")]
    NotNative,
    #[error("native handler field `crate_dir` is not valid UTF-8")]
    NonUtf8CrateDir,
    #[error("native handler prebuilt path for platform `{platform}` is not valid UTF-8")]
    NonUtf8PrebuiltPath { platform: String },
}

/// Compute the native implementation identity of one exact registry row.
///
/// A non-native row or an unrepresentable authored path returns its typed
/// refusal. The function deliberately accepts the whole row so both plan
/// lowering and artifact resolution bind to the same authored declaration
/// rather than reassembling the handler fields.
pub fn compiler_native_implementation_digest(
    row: &ExtensionRegistryRow,
) -> Result<CompilerNativeImplementationDigest, CompilerNativeImplementationDigestError> {
    let identity = NativeHandlerIdentity::from_handler(&row.declaration().handler)?
        .ok_or(CompilerNativeImplementationDigestError::NotNative)?;
    Ok(identity.digest())
}

/// The authored native-handler members retained by the private plan family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeHandlerIdentity {
    crate_dir: Option<String>,
    prebuilt: Option<BTreeMap<String, String>>,
}

impl NativeHandlerIdentity {
    #[cfg(test)]
    pub(crate) fn candidate(crate_dir: Option<&str>, prebuilt: Option<Vec<(&str, &str)>>) -> Self {
        Self {
            crate_dir: crate_dir.map(str::to_string),
            prebuilt: prebuilt.map(|pairs| {
                pairs
                    .into_iter()
                    .map(|(platform, path)| (platform.to_string(), path.to_string()))
                    .collect()
            }),
        }
    }

    pub(crate) fn from_handler(
        handler: &ExtensionHandler,
    ) -> Result<Option<Self>, CompilerNativeImplementationDigestError> {
        let ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } = handler
        else {
            return Ok(None);
        };
        let crate_dir = crate_dir
            .as_deref()
            .map(|path| portable_path(path.as_os_str(), NativePathSeat::CrateDir))
            .transpose()?;
        let prebuilt = prebuilt
            .as_ref()
            .map(|paths| {
                paths
                    .iter()
                    .map(|(platform, path)| {
                        portable_path(path.as_os_str(), NativePathSeat::Prebuilt(platform))
                            .map(|path| (platform.clone(), path))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()
            })
            .transpose()?;
        Ok(Some(Self {
            crate_dir,
            prebuilt,
        }))
    }

    pub(crate) fn digest(&self) -> CompilerNativeImplementationDigest {
        let mut digest = StableDigest::new(IMPLEMENTATION_DIGEST_DOMAIN);
        digest.byte(TAG_IMPLEMENTATION_NATIVE);
        digest.u32(NATIVE_ABI_EPOCH);
        digest.u32(COMPILER_IR_SCHEMA);
        match &self.crate_dir {
            Some(path) => {
                digest.byte(1);
                digest.field(path.as_bytes());
            }
            None => digest.byte(0),
        }
        match &self.prebuilt {
            Some(paths) => {
                digest.byte(1);
                digest.usize(paths.len());
                for (platform, path) in paths {
                    digest.field(platform.as_bytes());
                    digest.field(path.as_bytes());
                }
            }
            None => digest.byte(0),
        }
        CompilerNativeImplementationDigest(digest.finish())
    }
}

enum NativePathSeat<'platform> {
    CrateDir,
    Prebuilt(&'platform str),
}

/// The one authored-path projection used by every native identity member.
fn portable_path(
    path: &OsStr,
    seat: NativePathSeat<'_>,
) -> Result<String, CompilerNativeImplementationDigestError> {
    let text = path.to_str().ok_or_else(|| match seat {
        NativePathSeat::CrateDir => CompilerNativeImplementationDigestError::NonUtf8CrateDir,
        NativePathSeat::Prebuilt(platform) => {
            CompilerNativeImplementationDigestError::NonUtf8PrebuiltPath {
                platform: bounded_platform(platform),
            }
        }
    })?;
    Ok(text.replace('\\', "/"))
}

fn bounded_platform(platform: &str) -> String {
    const LIMIT: usize = 64;
    let mut characters = platform.chars();
    let mut bounded: String = characters.by_ref().take(LIMIT).collect();
    if characters.next().is_some() {
        bounded.push('…');
    }
    bounded
}
