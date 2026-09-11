use super::*;

fn entry(name: &str, bytes: &[u8], mode: u32) -> ArchiveEntry {
    ArchiveEntry {
        name: name.to_string(),
        bytes: bytes.to_vec(),
        mode,
    }
}

#[test]
fn output_is_deterministic_and_sorted() {
    let entries = [entry("z", b"last", 0o644), entry("a", b"first", 0o755)];
    let first = write_zip(&entries).unwrap();
    let second = write_zip(&entries).unwrap();
    assert_eq!(first, second);
    let read = read_zip(&first).unwrap();
    assert_eq!(
        read,
        [entry("a", b"first", 0o755), entry("z", b"last", 0o644)]
    );
}

#[test]
fn git_style_store_and_data_descriptor_entries_are_read() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new_stream(cursor);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);
    writer.start_file("Cargo.toml", options).unwrap();
    writer.write_all(b"[workspace]\n").unwrap();
    let bytes = writer.finish().unwrap().into_inner().into_inner();
    assert_eq!(
        read_zip(&bytes).unwrap(),
        [entry("Cargo.toml", b"[workspace]\n", 0o644)]
    );
}

#[test]
fn unsafe_and_duplicate_writer_paths_refuse() {
    for name in ["", "/root", "../escape", "a/./b", "a//b", "C:/file", "a\\b"] {
        assert!(write_zip(&[entry(name, b"x", 0o644)]).is_err(), "{name}");
    }
    assert!(write_zip(&[entry("same", b"1", 0o644), entry("same", b"2", 0o644)]).is_err());
}

#[test]
fn reader_refuses_unsafe_paths() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    writer
        .start_file("../escape", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"x").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    assert!(read_zip(&bytes).is_err());
}

#[test]
fn reader_refuses_crc_tampering() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    writer
        .start_file(
            "payload",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(b"known-payload").unwrap();
    let mut bytes = writer.finish().unwrap().into_inner();
    let offset = bytes
        .windows(b"known-payload".len())
        .position(|window| window == b"known-payload")
        .unwrap();
    bytes[offset] ^= 0xff;
    assert!(read_zip(&bytes).is_err());
}

#[test]
fn bounded_reader_rejects_expanded_size_from_metadata_before_materialising_entry() {
    let bytes = write_zip(&[entry("payload", &[0; 1024], 0o644)]).unwrap();
    let limits = BTreeMap::from([("payload".to_string(), 16)]);
    let error = read_zip_bounded(&bytes, &limits).unwrap_err().to_string();
    assert!(error.contains("1024 expanded bytes"));
    assert!(error.contains("16-byte limit"));
}
