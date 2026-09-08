//! Complete no-follow manifests and identity-bound owned-tree cleanup.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

use anyhow::{Result, bail};
use sha2::{Digest as _, Sha256};

use crate::file::identity::FileIdentity;
use crate::{Pinned, Project};

use super::{DirectoryDurability, identity_token, project_view, sync_directory};

include!("tree/model.rs");
include!("tree/ownership.rs");
include!("tree/pinned.rs");
include!("tree/internal.rs");
include!("tree/hooks.rs");
