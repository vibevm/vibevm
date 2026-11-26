//! Read only the aggregate member; the other attachments are opaque bytes.

use std::collections::BTreeSet;
use std::io::{Read, Seek, SeekFrom};

use anyhow::{Context, Result, bail, ensure};
use vibe_publish::release_manifest::{
    AggregateDistributionManifest, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
    DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME,
    DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES, DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES,
    DISTRIBUTION_TOOL_ASSET_MAX_BYTES, DISTRIBUTION_TOOLS_MANIFEST_FILENAME,
    DISTRIBUTION_TOOLS_MANIFEST_MAX_BYTES,
};
use vibe_publish::token::Token;

use super::super::download::{DownloadCleanup, cache_busted, download_path};
use super::super::{RemoteContext, current_target, refuse_remote_offline};
use super::ReleaseDir;

const ARCHIVE: &str = "vibevm-release-companion.zip";

/// Only a declared absence permits the archive lane, never a failed direct read.
pub(super) fn location(dir: &ReleaseDir) -> Option<&str> {
    match dir {
        ReleaseDir::Assets(assets)
            if !assets.contains_key(DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME) =>
        {
            assets.get(ARCHIVE).map(String::as_str)
        }
        _ => None,
    }
}

pub(in crate::commands::vvm::bundle) fn fetch_manifest(
    remote: &RemoteContext<'_>,
    dir: &ReleaseDir,
    token: Option<&Token>,
) -> Result<Option<AggregateDistributionManifest>> {
    let Some(url) = location(dir) else {
        return Ok(None);
    };
    refuse_remote_offline(remote, url)?;
    let task = remote.progress.task("Fetching release companion metadata");
    let path = download_path(remote.store, ARCHIVE);
    remote.store.guard_mutation_path(&path)?;
    let _cleanup = DownloadCleanup(path.clone());
    let result = (|| {
        remote.downloader.download_authorized(
            &cache_busted(url),
            &path,
            DISTRIBUTION_BUNDLE_MAX_BYTES,
            None,
            &task,
            token,
        )?;
        let bytes = read_member(std::fs::File::open(&path)?, Limits::default())?;
        AggregateDistributionManifest::from_json_slice_for_target(&bytes, current_target()?)
            .context("reading the companion distribution aggregate")
    })();
    if result.is_ok() {
        task.finish();
    } else {
        task.fail("release companion metadata was invalid");
    }
    result.map(Some)
}

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub(super) archive: u64,
    pub(super) total: u64,
    pub(super) aggregate: u64,
    pub(super) entries: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            archive: DISTRIBUTION_BUNDLE_MAX_BYTES,
            total: DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES,
            aggregate: DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
            entries: 256,
        }
    }
}

fn field<const N: usize>(bytes: &[u8], at: usize) -> Option<[u8; N]> {
    bytes.get(at..)?.first_chunk::<N>().copied()
}

fn word(bytes: &[u8], at: usize) -> Result<u16> {
    field(bytes, at)
        .map(u16::from_le_bytes)
        .context("truncated ZIP directory field")
}

fn dword(bytes: &[u8], at: usize) -> Result<u32> {
    field(bytes, at)
        .map(u32::from_le_bytes)
        .context("truncated ZIP directory field")
}

/// Preflight the complete central directory, before zip's name-keyed map can
/// hide duplicates or allocate from an unbounded declared entry count.
fn directory<R: Read + Seek>(reader: &mut R, limits: Limits) -> Result<usize> {
    let length = reader.seek(SeekFrom::End(0))?;
    ensure!(
        length <= limits.archive,
        "companion archive exceeds its compressed limit"
    );
    let tail_length = length.min(65_557) as usize;
    reader.seek(SeekFrom::End(-(tail_length as i64)))?;
    let mut tail = vec![0; tail_length];
    reader.read_exact(&mut tail)?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&at| {
            tail[at..at + 4] == *b"PK\x05\x06"
                && field(&tail, at + 20)
                    .is_some_and(|n| at + 22 + usize::from(u16::from_le_bytes(n)) == tail.len())
        })
        .context("companion has no valid ZIP end directory")?;
    ensure!(
        word(&tail, end + 4)? == 0 && word(&tail, end + 6)? == 0,
        "multi-disk companion ZIP is unsupported"
    );
    let count = word(&tail, end + 10)? as usize;
    ensure!(
        count <= limits.entries && word(&tail, end + 8)? as usize == count,
        "companion has too many entries or an inconsistent directory"
    );
    let size = dword(&tail, end + 12)? as u64;
    let start = dword(&tail, end + 16)? as u64;
    let end_position = length - tail_length as u64 + end as u64;
    ensure!(
        start.checked_add(size) == Some(end_position),
        "companion ZIP directory bounds are invalid or unsupported"
    );
    reader.seek(SeekFrom::Start(start))?;
    let mut names = BTreeSet::new();
    for _ in 0..count {
        let mut header = [0; 46];
        reader.read_exact(&mut header)?;
        ensure!(
            &header[..4] == b"PK\x01\x02",
            "invalid companion central directory entry"
        );
        let mut name = vec![0; word(&header, 28)? as usize];
        reader.read_exact(&mut name)?;
        ensure!(
            !name.is_empty()
                && name
                    .iter()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(c))
                && name != b"."
                && name != b".."
                && !name.ends_with(b".")
                && !name.starts_with(b"."),
            "unsafe companion entry name"
        );
        let name = String::from_utf8(name)?;
        ensure!(
            names.insert(name.to_ascii_lowercase()),
            "duplicate or colliding companion entry `{name}`"
        );
        reader.seek(SeekFrom::Current(
            i64::from(word(&header, 30)?) + i64::from(word(&header, 32)?),
        ))?;
        ensure!(
            reader.stream_position()? <= end_position,
            "companion directory entry exceeds directory bounds"
        );
    }
    ensure!(
        reader.stream_position()? == end_position,
        "companion directory count does not cover all entries"
    );
    Ok(count)
}

fn known_limit(name: &str) -> Option<u64> {
    if name == DISTRIBUTION_SOURCE_ARCHIVE_FILENAME {
        Some(DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES)
    } else if name == DISTRIBUTION_TOOLS_MANIFEST_FILENAME {
        Some(DISTRIBUTION_TOOLS_MANIFEST_MAX_BYTES)
    } else if name.starts_with("vibe-bootstrap-") {
        Some(DISTRIBUTION_BOOTSTRAP_MAX_BYTES)
    } else if name.starts_with("px-") && (name.ends_with(".zip") || name.ends_with(".tar.gz")) {
        Some(DISTRIBUTION_TOOL_ASSET_MAX_BYTES)
    } else {
        None
    }
}

pub(super) fn read_member<R: Read + Seek>(mut reader: R, limits: Limits) -> Result<Vec<u8>> {
    let count = directory(&mut reader, limits)?;
    reader.rewind()?;
    let mut zip = zip::ZipArchive::new(reader).context("opening companion ZIP")?;
    ensure!(zip.len() == count, "companion directory lost entries");
    let directory_start = zip.central_directory_start();
    let mut total = 0_u64;
    let mut aggregate = None;
    for index in 0..zip.len() {
        // Raw access checks headers without constructing a decompressor or
        // reading any attachment contents. Only the aggregate is read below.
        let member = zip.by_index_raw(index)?;
        ensure!(
            member.is_file()
                && member.unix_mode().is_none_or(|mode| {
                    let kind = mode & 0o170000;
                    kind == 0 || kind == 0o100000
                }),
            "companion entry is not an ordinary file"
        );
        ensure!(
            !member.encrypted()
                && matches!(
                    member.compression(),
                    zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
                ),
            "unsupported companion entry format"
        );
        ensure!(
            member
                .data_start()
                .checked_add(member.compressed_size())
                .is_some_and(|end| end <= directory_start),
            "companion member exceeds data bounds"
        );
        total = total
            .checked_add(member.size())
            .context("companion expanded size overflow")?;
        ensure!(
            total <= limits.total,
            "companion expanded total exceeds its limit"
        );
        let maximum = if member.name() == DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME {
            aggregate = Some(index);
            Some(limits.aggregate)
        } else {
            known_limit(member.name())
        };
        if let Some(maximum) = maximum {
            ensure!(
                member.size() <= maximum,
                "companion member `{}` exceeds its limit",
                member.name()
            );
        }
    }
    let index = aggregate.context("companion carries no distribution aggregate")?;
    let mut member = zip.by_index(index)?;
    let declared = member.size();
    let mut bytes = Vec::new();
    (&mut member)
        .take(limits.aggregate + 1)
        .read_to_end(&mut bytes)
        .context("reading companion aggregate and verifying its CRC")?;
    if bytes.len() as u64 > limits.aggregate || bytes.len() as u64 != declared {
        bail!("companion aggregate actual size exceeds its limit or differs from its declaration");
    }
    Ok(bytes)
}
