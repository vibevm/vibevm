//! The request envelope: which band goes in which API role, in what order,
//! exactly once.
//!
//! **A filename chooses no API role.** `SYSTEM.md` is a name on a disk, and
//! a text separator inside one message creates no role boundary. The
//! adapter therefore *places* the project's trusted instructions in a named
//! instruction role and says in the envelope which role it chose, so the
//! elevation is acknowledged rather than assumed away: project instructions,
//! package boot text, examples and operational grants acquire no
//! provider-level authority by sitting in a file (`SP-FMT-011`).
//!
//! **No double injection, and that is the one failure this feature most
//! plausibly produces.** The prefix already contains the entrypoint, so an
//! adapter that also auto-loaded the entrypoint into the first message
//! would double the cost this feature exists to reduce and leave the model
//! two copies of the same instructions. Band T is the caller's own and is
//! the *only* message content the envelope carries; nothing is loaded by
//! reference, nothing is appended "for convenience", and the test is a
//! substring count over a captured request (`#SYS-NO-DOUBLE-INJECTION`).
//!
//! **Nothing is sent from here.** The envelope is a document. This cell
//! assembles no credentials, chooses no model, opens no connection and
//! reaches no provider; the only provider data it ever handles is a usage
//! record a caller already received and saved (`usage.rs`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-REQUEST-ENVELOPE");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_workspace::system_prompt::{PreparedBand, PreparedRequest};

pub(crate) mod usage;

pub(crate) use usage::{CacheDiagnostics, ProviderUsage};

/// Which band a piece of the request belongs to.
///
/// The letters are the design's own (`PROP-076`): **P** the published
/// prefix, **U** the unconditional continuation rows, **G** the rows a
/// session target selects, **T** the caller's task. The order is the order
/// of the boot law, which is also the cache-optimal order — a coincidence
/// the design records so nobody "optimises" it apart again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Band {
    /// Band P — the published prefix, as one block.
    Prefix,
    /// Band U — every unconditional continuation row, in plan order.
    Unconditional,
    /// Band G — the gated rows this session's target selects.
    Gated,
    /// Band T — the caller's own task content, which vibe never witnesses.
    Task,
}

impl Band {
    /// The band's letter, which is how every report spells it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Band::Prefix => "P",
            Band::Unconditional => "U",
            Band::Gated => "G",
            Band::Task => "T",
        }
    }
}

/// The API role a block is placed in — named, never inferred from a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// A supported instruction role: `system`, `developer`, or whichever
    /// field the provider gives that standing. The envelope names the
    /// *kind* of role; which field carries it is the runner's mapping.
    Instruction,
    /// An ordinary message, which is the only role band T may take.
    Message,
}

impl Role {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Role::Instruction => "instruction",
            Role::Message => "message",
        }
    }
}

/// One instruction block, with the bytes it was built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    /// Which band it belongs to.
    pub(crate) band: Band,
    /// The API role the adapter placed it in.
    pub(crate) role: Role,
    /// The path the plan spells for it.
    pub(crate) path: String,
    /// SHA-256 of the bytes in `text`, which the preparation also observed.
    pub(crate) sha256: String,
    /// Byte offset from the first instruction byte.
    pub(crate) offset: usize,
    /// The bytes themselves.
    pub(crate) text: String,
}

/// One band-T message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    /// Always `Band::Task`: a message is never a band the prefix owns.
    pub(crate) band: Band,
    /// Always `Role::Message`.
    pub(crate) role: Role,
    /// The caller's own text.
    pub(crate) text: String,
}

/// A tool the request declares. vibe contributes none, and the type exists
/// so the tool block's *position* is a stated fact rather than an omission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolRef {
    /// The tool's name as the request declares it.
    pub(crate) name: String,
}

/// Where the tool block sits relative to the instruction blocks.
///
/// Ahead of them, because every provider whose cache hierarchy was checked
/// puts tools first: a changed tool set invalidates the instruction blocks
/// behind it, and a runner that placed tools after the prefix would be
/// paying for a cache entry it breaks on every tool edit
/// (`#SYS-CACHE-FACTS`).
pub(crate) const TOOLS_POSITION: &str = "before_instructions";

/// One request, as roles and order rather than as one string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestEnvelope {
    /// Bands P, U and G, in that order, each in an instruction role.
    pub(crate) instruction_blocks: Vec<Block>,
    /// The tools the request declares — empty, and declared so.
    pub(crate) tools: Vec<ToolRef>,
    /// Band T, after every instruction block.
    pub(crate) messages: Vec<Message>,
    /// Recommended cache-boundary offsets, counted from the first
    /// instruction byte.
    pub(crate) cache_boundaries: Vec<usize>,
}

impl RequestEnvelope {
    /// Build the envelope for one prepared request.
    ///
    /// **Each band's bytes are admitted only against the digest the
    /// preparation observed.** A file edited between `prepare` and here is
    /// a refusal naming it, not a silently different request: the whole
    /// point of a witnessed band is that the bytes sent are the bytes
    /// measured, and a digest that no longer matches means the two are no
    /// longer the same request.
    pub(crate) fn build(
        prepared: &PreparedRequest,
        prefix_path: &Path,
        workspace_root: &Path,
        task: Option<&str>,
    ) -> Result<Self> {
        let mut blocks = Vec::new();
        let mut offset = 0_usize;
        let mut push = |band: Band, member: &PreparedBand, file: PathBuf| -> Result<()> {
            let text = admit(member, &file)?;
            offset += text.len();
            blocks.push(Block {
                band,
                role: Role::Instruction,
                path: member.path.clone(),
                sha256: member.sha256.clone(),
                offset: offset - text.len(),
                text,
            });
            Ok(())
        };
        push(Band::Prefix, &prepared.prefix, prefix_path.to_path_buf())?;
        for member in &prepared.unconditional {
            push(
                Band::Unconditional,
                member,
                workspace_root.join(&member.path),
            )?;
        }
        for member in &prepared.gated {
            push(Band::Gated, member, workspace_root.join(&member.path))?;
        }
        Ok(Self {
            instruction_blocks: blocks,
            tools: Vec::new(),
            // Band T is the caller's, so an absent task is an empty
            // message list and never a placeholder vibe invented.
            messages: task
                .map(|text| Message {
                    band: Band::Task,
                    role: Role::Message,
                    text: text.to_owned(),
                })
                .into_iter()
                .collect(),
            cache_boundaries: prepared.cache_boundaries.clone(),
        })
    }

    /// The total instruction bytes, which is what every boundary offset is
    /// measured against.
    pub(crate) fn instruction_len(&self) -> usize {
        self.instruction_blocks
            .iter()
            .map(|block| block.text.len())
            .sum()
    }
}

/// Read one band's bytes and admit them only if they still digest to what
/// the preparation witnessed.
fn admit(member: &PreparedBand, file: &Path) -> Result<String> {
    let bytes = std::fs::read(file).with_context(|| {
        format!(
            "reading band member `{}` from `{}`",
            member.path,
            file.display()
        )
    })?;
    let digest = vibe_workspace::system_prompt::sha256_hex(&bytes);
    if digest != member.sha256 {
        bail!(
            "`{}` changed between preparation and envelope construction — prepared {}, now {}; \
             the bytes sent must be the bytes measured, so this request is refused rather than \
             silently sent as a different one",
            file.display(),
            &member.sha256[..12.min(member.sha256.len())],
            &digest[..12.min(digest.len())]
        );
    }
    String::from_utf8(bytes).with_context(|| {
        format!(
            "`{}` is not UTF-8, and an instruction block is text",
            file.display()
        )
    })
}

#[cfg(test)]
#[path = "adapter/tests.rs"]
mod tests;
