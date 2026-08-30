//! Closed vocabulary for the three client-plugin destination rows.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::mechanism::deploy::model::{ClientExecutable, ClientExecutables};
use crate::mechanism::{
    BUILTIN_CLAUDE_PLUGIN_PIN, BUILTIN_CODEX_PLUGIN_PIN, BUILTIN_OPENCODE_PLUGIN_PIN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginClient {
    Claude,
    Codex,
    OpenCode,
}

pub(super) struct CliIdentity {
    pub(super) marketplace: String,
    pub(super) coordinate: String,
    pub(super) resource: String,
    pub(super) version: String,
    pub(super) desired_digest: String,
}

pub(super) fn framed_hash(domain: &str, fields: &[(&str, &str)]) -> String {
    let mut hash = Sha256::new();
    hash.update(domain.as_bytes());
    hash.update(b"\x00");
    for (name, value) in fields {
        hash.update(name.as_bytes());
        hash.update(b"\x00");
        hash.update(value.as_bytes());
        hash.update(b"\x00");
    }
    format!("{:x}", hash.finalize())
}

impl PluginClient {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 3] = [Self::Claude, Self::Codex, Self::OpenCode];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }

    pub(crate) const fn pin(self) -> &'static str {
        match self {
            Self::Claude => BUILTIN_CLAUDE_PLUGIN_PIN,
            Self::Codex => BUILTIN_CODEX_PLUGIN_PIN,
            Self::OpenCode => BUILTIN_OPENCODE_PLUGIN_PIN,
        }
    }

    pub(crate) const fn manifest_dir(self) -> Option<&'static str> {
        match self {
            Self::Claude => Some(".claude-plugin"),
            Self::Codex => Some(".codex-plugin"),
            Self::OpenCode => None,
        }
    }

    pub(crate) const fn supported_version(self) -> &'static str {
        match self {
            Self::Claude => "2.1.x",
            Self::Codex => "0.148.x",
            Self::OpenCode => "1.17.x",
        }
    }

    pub(crate) const fn version_pair(self) -> (u64, u64) {
        match self {
            Self::Claude => (2, 1),
            Self::Codex => (0, 148),
            Self::OpenCode => (1, 17),
        }
    }

    pub(crate) fn executable(self, clients: &ClientExecutables) -> &ClientExecutable {
        match self {
            Self::Claude => &clients.claude,
            Self::Codex => &clients.codex,
            Self::OpenCode => &clients.opencode,
        }
    }

    pub(crate) fn private_root(self, home: &Path) -> Option<(&'static str, PathBuf)> {
        match self {
            Self::Claude => Some(("CLAUDE_CONFIG_DIR", home.join(".claude"))),
            Self::Codex => Some(("CODEX_HOME", home.join(".codex"))),
            Self::OpenCode => None,
        }
    }

    pub(crate) const fn logical_lock(self) -> &'static str {
        match self {
            Self::Claude => "client-state:claude:plugins",
            Self::Codex => "client-state:codex:plugins",
            Self::OpenCode => "home:.config/opencode/opencode.json",
        }
    }
}
