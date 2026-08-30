//! The deterministic ZIP writer — §7.0.8's acceptance, as an algorithm.
//!
//! > "a byte-identical archive on re-run: entries sorted by archived name,
//! > forward-slash names, one fixed timestamp constant, fixed compression
//! > parameters, no platform extra fields".
//!
//! Every one of those is a decision this file makes ONCE, and the reason
//! each is a constant rather than a knob is the same: the acceptance is
//! that two runs produce one digest, and every value that could differ
//! between two runs is a way for that to stop being true.
//!
//! - **Entry order** is the caller's sorted census (`contain::walk_tree`
//!   and the input order), and the writer re-sorts nothing: it refuses an
//!   unsorted or duplicated census rather than quietly repairing it, so a
//!   caller cannot lose the property by accident.
//! - **Names** are forward-slashed, relative, and never carry a `.` or
//!   `..` component. The general-purpose bit 11 (UTF-8 name) is set
//!   exactly when the name is not ASCII — a function of the name, so still
//!   a constant per archive.
//! - **Timestamps** are the fixed MS-DOS epoch 1980-01-01 00:00:00. The
//!   files' real mtimes are deliberately not read: an archive that carried
//!   them would differ between two runs that produced identical content,
//!   which is precisely the failure this provider exists to not have.
//! - **Compression** is STORED (method 0) for every entry. That is a fixed
//!   compression parameter in the strongest available sense: DEFLATE
//!   output is a property of the compressor's version and heuristics, so
//!   an archive compressed by two different builds of one library is two
//!   different archives with one content. Storing is the only choice whose
//!   determinism this engine can actually prove, and proving it is the
//!   acceptance.
//! - **Extra fields** are empty and the external attributes are zero, so
//!   no platform's permission bits, timestamps or alignment padding enter
//!   the bytes.
//!
//! ZIP64 is deliberately not written: an entry or an archive at or beyond
//! 4 GiB refuses by name rather than silently emitting a format this
//! writer cannot claim to have produced deterministically.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

/// The fixed MS-DOS date: 1980-01-01. `year-1980 << 9 | month << 5 | day`.
const DOS_DATE: u16 = (1 << 5) | 1;
/// The fixed MS-DOS time: 00:00:00.
const DOS_TIME: u16 = 0;
/// STORED — no compression, and the one method this writer emits.
const METHOD_STORED: u16 = 0;
/// Version needed to extract a stored entry: 1.0.
const VERSION_NEEDED: u16 = 10;
/// Version made by: 2.0, host 0 (MS-DOS/FAT) — no platform attributes.
const VERSION_MADE_BY: u16 = 20;
/// General-purpose bit 11: the name and comment are UTF-8.
const FLAG_UTF8: u16 = 0x0800;
/// The largest value the non-ZIP64 headers can carry — `u32::MAX`,
/// spelled as a literal because a `const` context cannot widen it.
const SIZE_CEILING: u64 = 0xFFFF_FFFF;

/// Why an archive could not be written deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchiveFault {
    /// The entry census is not in archived-name order, or repeats a name.
    /// Both are the same defect — the writer's determinism is a property
    /// of the census, so it verifies rather than assumes it.
    Census { detail: String },
    /// One entry name is not a portable archive name.
    Name { name: String, reason: String },
    /// A size no non-ZIP64 header can carry.
    TooLarge { name: String, bytes: u64 },
}

impl ArchiveFault {
    /// The one-clause reason a refusal quotes.
    pub(crate) fn reason(&self) -> String {
        match self {
            Self::Census { detail } => format!(
                "the archive entry census is not usable: {detail}; entries are archived in sorted \
                 name order and each name appears once"
            ),
            Self::Name { name, reason } => {
                format!("archive entry name `{name}` is not portable: {reason}")
            }
            Self::TooLarge { name, bytes } => format!(
                "archive entry `{name}` holds {bytes} byte(s), which no non-ZIP64 header can \
                 carry; this writer emits only the deterministic classic format"
            ),
        }
    }
}

/// One entry to archive: its archived name and its exact bytes.
pub(crate) struct ArchiveEntry<'a> {
    /// Forward-slashed, relative, no `.` or `..` component.
    pub(crate) name: &'a str,
    pub(crate) bytes: &'a [u8],
}

/// Render one archive.
///
/// Pure: it opens nothing and reads no clock. The bytes in are the bytes
/// out, so two calls with equal input are equal — which is the property
/// the two-runs-one-digest acceptance rests on, provable without a
/// filesystem.
pub(crate) fn write_archive(entries: &[ArchiveEntry<'_>]) -> Result<Vec<u8>, ArchiveFault> {
    census_is_canonical(entries)?;
    // The classic EOCD carries the entry count in a u16, and 0xFFFF is the
    // ZIP64 sentinel some readers chase — so the last classic count is
    // 65535 and one more refuses, exactly as an oversized entry does.
    if entries.len() > usize::from(u16::MAX) {
        return Err(ArchiveFault::Census {
            detail: format!(
                "{} entries; a classic ZIP central directory carries at most {}",
                entries.len(),
                u16::MAX
            ),
        });
    }
    let mut body: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    let mut count: u16 = 0;
    for entry in entries {
        check_name(entry.name)?;
        let length = entry.bytes.len() as u64;
        // `>=`: the exact value 0xFFFF_FFFF is the ZIP64 sentinel in a
        // size/offset field, so the largest classic value is one below it.
        if length >= SIZE_CEILING {
            return Err(ArchiveFault::TooLarge {
                name: entry.name.to_owned(),
                bytes: length,
            });
        }
        let offset = body.len() as u64;
        if offset >= SIZE_CEILING {
            return Err(ArchiveFault::TooLarge {
                name: entry.name.to_owned(),
                bytes: offset,
            });
        }
        let crc = crc32(entry.bytes);
        let flags = if entry.name.is_ascii() { 0 } else { FLAG_UTF8 };
        let name = entry.name.as_bytes();
        let size = length as u32;

        push_u32(&mut body, 0x0403_4b50);
        push_u16(&mut body, VERSION_NEEDED);
        push_u16(&mut body, flags);
        push_u16(&mut body, METHOD_STORED);
        push_u16(&mut body, DOS_TIME);
        push_u16(&mut body, DOS_DATE);
        push_u32(&mut body, crc);
        push_u32(&mut body, size);
        push_u32(&mut body, size);
        push_u16(&mut body, name.len() as u16);
        push_u16(&mut body, 0);
        body.extend_from_slice(name);
        body.extend_from_slice(entry.bytes);

        push_u32(&mut directory, 0x0201_4b50);
        push_u16(&mut directory, VERSION_MADE_BY);
        push_u16(&mut directory, VERSION_NEEDED);
        push_u16(&mut directory, flags);
        push_u16(&mut directory, METHOD_STORED);
        push_u16(&mut directory, DOS_TIME);
        push_u16(&mut directory, DOS_DATE);
        push_u32(&mut directory, crc);
        push_u32(&mut directory, size);
        push_u32(&mut directory, size);
        push_u16(&mut directory, name.len() as u16);
        // Extra field, file comment, disk number, internal attributes and
        // external attributes: all zero. That is the "no platform extra
        // fields" law, spelled as five constants rather than as prose.
        push_u16(&mut directory, 0);
        push_u16(&mut directory, 0);
        push_u16(&mut directory, 0);
        push_u16(&mut directory, 0);
        push_u32(&mut directory, 0);
        push_u32(&mut directory, offset as u32);
        directory.extend_from_slice(name);

        count = count.saturating_add(1);
    }
    let directory_offset = body.len() as u64;
    let directory_size = directory.len() as u64;
    if directory_offset >= SIZE_CEILING || directory_size >= SIZE_CEILING {
        return Err(ArchiveFault::TooLarge {
            name: "<central directory>".to_owned(),
            bytes: directory_offset.max(directory_size),
        });
    }
    let mut archive = body;
    archive.extend_from_slice(&directory);
    push_u32(&mut archive, 0x0605_4b50);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, count);
    push_u16(&mut archive, count);
    push_u32(&mut archive, directory_size as u32);
    push_u32(&mut archive, directory_offset as u32);
    push_u16(&mut archive, 0);
    Ok(archive)
}

/// The census is sorted by archived name and holds each name once.
fn census_is_canonical(entries: &[ArchiveEntry<'_>]) -> Result<(), ArchiveFault> {
    for pair in entries.windows(2) {
        let [left, right] = pair else { continue };
        if left.name == right.name {
            return Err(ArchiveFault::Census {
                detail: format!("`{}` appears twice", left.name),
            });
        }
        if left.name.as_bytes() > right.name.as_bytes() {
            return Err(ArchiveFault::Census {
                detail: format!("`{}` sorts after `{}`", left.name, right.name),
            });
        }
    }
    Ok(())
}

/// One archived name's portability law.
fn check_name(name: &str) -> Result<(), ArchiveFault> {
    let refuse = |reason: &str| ArchiveFault::Name {
        name: name.to_owned(),
        reason: reason.to_owned(),
    };
    if name.is_empty() {
        return Err(refuse("it names nothing"));
    }
    if name.len() > usize::from(u16::MAX) {
        return Err(refuse("it is longer than a ZIP name header can carry"));
    }
    if name.contains('\\') {
        return Err(refuse("archived names are forward-slashed"));
    }
    if name.starts_with('/') {
        return Err(refuse("an archived name is relative to the archive root"));
    }
    if name.ends_with('/') {
        return Err(refuse(
            "this writer archives regular files; a trailing slash names a directory entry",
        ));
    }
    if name.split('/').any(|part| part.is_empty()) {
        return Err(refuse("it holds an empty path segment"));
    }
    if name.split('/').any(|part| part == "." || part == "..") {
        return Err(refuse("a `.` or `..` segment has no single meaning"));
    }
    if name.chars().any(char::is_control) {
        return Err(refuse("it holds a control byte"));
    }
    Ok(())
}

/// CRC-32 (IEEE 802.3, reflected, polynomial `0xEDB88320`) — the checksum
/// every ZIP header carries.
///
/// Written here rather than pulled in: the workspace declares no CRC
/// dependency, and the bit-at-a-time reflected form is fourteen lines
/// whose only correctness obligation — the standard `"123456789"` check
/// value `0xCBF4_3926` — is a test one line long.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let carry = crc & 1;
            crc >>= 1;
            if carry != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    !crc
}

fn push_u16(sink: &mut Vec<u8>, value: u16) {
    sink.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(sink: &mut Vec<u8>, value: u32) {
    sink.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
