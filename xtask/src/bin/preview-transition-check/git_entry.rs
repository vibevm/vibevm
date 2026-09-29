//! Shared borrowed decoder for immutable Git ls-tree rows.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CAPTURE-GIT-READ-BOUNDARY");

pub(super) struct Entry<'a> {
    pub mode: &'a str,
    pub kind: &'a str,
    pub oid: &'a str,
    pub repository_path: &'a str,
}

impl Entry<'_> {
    pub fn regular(&self) -> bool {
        matches!(self.mode, "100644" | "100755") && self.kind == "blob"
    }
}

pub(super) enum DecodeError {
    NoPathDelimiter,
    MetadataEncoding,
    MalformedMetadata,
    PathEncoding,
}

impl DecodeError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::NoPathDelimiter => "Git census row has no path delimiter",
            Self::MetadataEncoding => "Git census metadata cannot be represented",
            Self::MalformedMetadata => "Git census metadata is malformed",
            Self::PathEncoding => "Captured path is not representable by the UTF-8 wire format",
        }
    }
}

pub(super) fn rows(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes.split(|byte| *byte == 0).filter(|row| !row.is_empty())
}

pub(super) fn decode(row: &[u8]) -> Result<Entry<'_>, DecodeError> {
    let separator = row
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or(DecodeError::NoPathDelimiter)?;
    let metadata =
        std::str::from_utf8(&row[..separator]).map_err(|_| DecodeError::MetadataEncoding)?;
    let mut parts = metadata.split(' ');
    let (Some(mode), Some(kind), Some(oid), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(DecodeError::MalformedMetadata);
    };
    let repository_path =
        std::str::from_utf8(&row[separator + 1..]).map_err(|_| DecodeError::PathEncoding)?;
    Ok(Entry {
        mode,
        kind,
        oid,
        repository_path,
    })
}
