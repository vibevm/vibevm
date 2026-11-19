//! The distribution reader's existing no-follow regular-file primitive.
use std::{fs, io, path::Path};

pub fn open_regular_no_follow(path: &Path) -> io::Result<(fs::File, fs::Metadata)> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "linux")]
        options.custom_flags(0x2_0000);
        #[cfg(target_os = "macos")]
        options.custom_flags(0x100);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = false;
    if !metadata.file_type().is_file() || reparse {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "distribution input is not a regular non-link file",
        ));
    }
    Ok((file, metadata))
}
