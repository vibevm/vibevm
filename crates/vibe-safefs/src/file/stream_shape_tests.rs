//! A structural fence on the streaming cell's source.
//!
//! "Streams without retaining the file" is a memory law, and a memory law that
//! only behaviour tests defend is one a later refactor can lose silently: a
//! whole-file buffer passes every digest test in this crate, exactly, and only
//! fails on the multi-gigabyte artifact nobody puts in a test suite. So the
//! ban is read off the source itself.
//!
//! This does not pretend to measure resident memory. It pins the two
//! constructs that turn a stream into a buffer, and nothing more: the answer
//! to "is it bounded" stays a review of this one small file, which is why the
//! file is small.

/// The production cell only — never its tests, which legitimately build whole
/// inputs to hash independently.
const STREAM_SOURCE: &str = include_str!("stream.rs");

#[test]
fn the_streaming_cell_never_reads_a_whole_file_into_memory() {
    for banned in ["read_to_end", "read_to_string"] {
        assert!(
            !STREAM_SOURCE.contains(banned),
            "`{banned}` sizes an allocation by the file, which is the one thing a witness \
             over a multi-gigabyte artifact may not do",
        );
    }
}

#[test]
fn the_streaming_cell_holds_no_growable_content_buffer() {
    for banned in ["Vec<u8>", "Vec::with_capacity", "to_vec()", "Vec::new()"] {
        assert!(
            !STREAM_SOURCE.contains(banned),
            "`{banned}` in the streaming cell would make peak memory a function of the \
             artifact's size; the window plus a 32-byte digest is the whole budget",
        );
    }
}

/// The window has to be a fixed-size array, not a runtime-sized allocation
/// that merely happens to be small today.
#[test]
fn the_streaming_cell_reads_through_one_fixed_stack_window() {
    assert!(
        STREAM_SOURCE.contains("[0_u8; READ_CHUNK]"),
        "the read window must be a fixed-size array of the shared chunk constant",
    );
    assert!(
        STREAM_SOURCE.contains("checked_add"),
        "the byte count must be checked, so a wrap understates rather than refuses",
    );
}

/// Both passes must go through the same held handle. A second `open_with` in
/// this cell would mean the second pass answered for whatever the name holds
/// now, which is the substitution the held handle exists to prevent.
#[test]
fn the_streaming_cell_opens_the_object_exactly_once() {
    assert_eq!(
        STREAM_SOURCE.matches("open_with").count(),
        1,
        "the second pass must seek the handle already open, never re-open the name",
    );
    assert!(
        STREAM_SOURCE.contains("SeekFrom::Start(0)"),
        "the second pass must rewind the held handle",
    );
}
