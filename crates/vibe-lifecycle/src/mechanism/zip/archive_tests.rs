//! The deterministic writer's own laws, provable without a filesystem.

use specmark::verifies;

use super::{ArchiveEntry, ArchiveFault, crc32, write_archive};

fn entry<'a>(name: &'a str, bytes: &'a [u8]) -> ArchiveEntry<'a> {
    ArchiveEntry { name, bytes }
}

/// The standard CRC-32 check value. Fourteen hand-written lines earn
/// exactly one obligation, and this is it.
#[test]
fn the_checksum_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b""), 0);
}

/// §7.0.8's acceptance, at the level where it is a property of a pure
/// function: equal input, equal bytes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn two_renderings_of_one_census_are_byte_identical() {
    let entries = [entry("a.txt", b"alpha"), entry("b/c.txt", b"gamma")];
    let first = write_archive(&entries).expect("the archive renders");
    let second = write_archive(&entries).expect("the archive renders again");
    assert_eq!(first, second);
    assert!(
        first.starts_with(&[0x50, 0x4b, 0x03, 0x04]),
        "a local header"
    );
    assert!(
        first
            .windows(4)
            .any(|window| window == [0x50, 0x4b, 0x05, 0x06]),
        "and an end-of-central-directory record",
    );
}

/// The timestamp is a CONSTANT: the two date/time fields of every local
/// header are the fixed 1980-01-01 00:00:00, whatever the clock says.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn every_entry_carries_the_one_fixed_timestamp_and_no_extra_field() {
    let archive = write_archive(&[entry("a.txt", b"alpha")]).expect("the archive renders");
    // Local header: time at offset 10, date at 12, extra length at 28.
    assert_eq!(u16::from_le_bytes([archive[10], archive[11]]), 0);
    assert_eq!(u16::from_le_bytes([archive[12], archive[13]]), 0x0021);
    assert_eq!(u16::from_le_bytes([archive[8], archive[9]]), 0, "STORED");
    assert_eq!(
        u16::from_le_bytes([archive[28], archive[29]]),
        0,
        "no platform extra field",
    );
}

/// An unsorted census refuses rather than being quietly repaired: the
/// determinism is a property of the caller's own order, and a writer that
/// fixed it silently would hide a caller that had lost it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_unsorted_or_repeated_census_refuses() {
    let unsorted = write_archive(&[entry("b.txt", b"b"), entry("a.txt", b"a")])
        .expect_err("an unsorted census refuses");
    let ArchiveFault::Census { detail } = &unsorted else {
        panic!("expected a census refusal, got: {unsorted:?}");
    };
    assert!(detail.contains("sorts after"), "{detail}");

    let repeated = write_archive(&[entry("a.txt", b"a"), entry("a.txt", b"a")])
        .expect_err("a repeated name refuses");
    let ArchiveFault::Census { detail } = &repeated else {
        panic!("expected a census refusal, got: {repeated:?}");
    };
    assert!(detail.contains("appears twice"), "{detail}");
}

/// A census past the classic u16 entry ceiling refuses rather than
/// saturating the EOCD count — 0xFFFF there is the ZIP64 sentinel, and
/// this writer refuses to emit a format it cannot claim.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_census_past_the_classic_entry_ceiling_refuses() {
    let names: Vec<String> = (0..=u16::MAX as u32).map(|i| format!("{i:05}")).collect();
    let entries: Vec<ArchiveEntry<'_>> =
        names.iter().map(|name| entry(name.as_str(), b"")).collect();
    let error = write_archive(&entries).expect_err("65536 entries refuse");
    let ArchiveFault::Census { detail } = &error else {
        panic!("expected a census refusal, got: {error:?}");
    };
    assert!(detail.contains("at most 65535"), "{detail}");
    // One fewer is the largest classic archive and still writes.
    write_archive(&entries[..entries.len() - 1]).expect("65535 entries write");
}

/// The archive is readable by an INDEPENDENT extractor — the one oracle a
/// hand-rolled writer owes that its own header reader cannot provide. A
/// wrong CRC or a malformed header is invisible to a self-read and fatal
/// to every real consumer, so the consumer is the pin: Windows'
/// `Expand-Archive` (System.IO.Compression) extracts the bytes and
/// verifies each entry's checksum while doing it.
#[test]
#[cfg(windows)]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_archive_is_readable_by_an_independent_extractor() {
    let archive = write_archive(&[
        entry("distribution/windows/helper.exe", b"the helper bytes"),
        entry("readme.txt", b"read me\n"),
    ])
    .expect("the archive writes");
    let scratch = tempfile::TempDir::new().expect("a scratch home");
    let zip = scratch.path().join("distributable.zip");
    std::fs::write(&zip, &archive).expect("the archive lands on disk");
    let out = scratch.path().join("extracted");
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Expand-Archive -LiteralPath '{}' -DestinationPath '{}'",
                zip.display(),
                out.display()
            ),
        ])
        .status()
        .expect("powershell runs");
    assert!(
        status.success(),
        "the independent extractor accepts the archive"
    );
    assert_eq!(
        std::fs::read(out.join("distribution/windows/helper.exe")).expect("the nested entry"),
        b"the helper bytes"
    );
    assert_eq!(
        std::fs::read(out.join("readme.txt")).expect("the flat entry"),
        b"read me\n"
    );
}

/// Every unportable archived name refuses, each naming its own reason.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn each_unportable_name_names_its_own_reason() {
    for (name, needle) in [
        ("a\\b.txt", "forward-slashed"),
        ("/a.txt", "relative to the archive root"),
        ("a/", "trailing slash"),
        ("a//b", "empty path segment"),
        ("a/../b", "`.` or `..`"),
        ("", "names nothing"),
    ] {
        let fault = write_archive(&[entry(name, b"x")])
            .expect_err(&format!("`{name}` is not a portable archived name"));
        assert!(
            fault.reason().contains(needle),
            "`{name}` refused with `{}`, expected `{needle}`",
            fault.reason(),
        );
    }
}

/// A non-ASCII name sets the UTF-8 general-purpose bit — a function of
/// the name, so the archive stays a function of its census.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_non_ascii_name_declares_its_encoding() {
    let ascii = write_archive(&[entry("a.txt", b"x")]).expect("the archive renders");
    assert_eq!(u16::from_le_bytes([ascii[6], ascii[7]]), 0);
    let wide = write_archive(&[entry("привет.txt", b"x")]).expect("the archive renders");
    assert_eq!(u16::from_le_bytes([wide[6], wide[7]]), 0x0800);
}
