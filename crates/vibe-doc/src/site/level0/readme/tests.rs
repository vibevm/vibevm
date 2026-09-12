//! The three block shapes, and the ones this reader admits it does not
//! understand.

use super::*;

#[test]
fn a_heading_opens_a_block_and_a_hashtag_does_not() {
    assert_eq!(
        blocks("# One\n\n## Two ##\n\n#notaheading\n"),
        vec![
            Block::Heading { text: "One".into() },
            Block::Heading { text: "Two".into() },
            Block::Paragraph {
                text: "#notaheading".into()
            },
        ]
    );
}

#[test]
fn a_paragraph_is_its_lines_joined_and_its_markdown_untouched() {
    assert_eq!(
        blocks("a `code` line\nand its second\n\nanother\n"),
        vec![
            Block::Paragraph {
                text: "a `code` line and its second".into()
            },
            Block::Paragraph {
                text: "another".into()
            },
        ]
    );
}

#[test]
fn a_fence_keeps_its_language_and_its_lines() {
    assert_eq!(
        blocks("```rust\nlet x = 1;\n\nlet y = 2;\n```\n"),
        vec![Block::Fence {
            lang: Some("rust".into()),
            body: "let x = 1;\n\nlet y = 2;".into()
        }]
    );
}

#[test]
fn a_fence_with_no_language_declares_none() {
    assert_eq!(
        blocks("```\nplain\n```\n"),
        vec![Block::Fence {
            lang: None,
            body: "plain".into()
        }]
    );
}

/// A heading inside a fence is code, not a heading — which is the one
/// place a line-by-line reader would go wrong on a README full of shell
/// examples.
#[test]
fn a_hash_inside_a_fence_is_code() {
    assert_eq!(
        blocks("```sh\n# a comment\nvibe install\n```\n"),
        vec![Block::Fence {
            lang: Some("sh".into()),
            body: "# a comment\nvibe install".into()
        }]
    );
}

/// Dropping the end of a file over three missing characters would lose
/// text somebody wrote.
#[test]
fn a_fence_nobody_closed_is_still_read() {
    assert_eq!(
        blocks("```\nunclosed\n"),
        vec![Block::Fence {
            lang: None,
            body: "unclosed".into()
        }]
    );
}

/// The stated limit, pinned so it stays a decision rather than becoming
/// a surprise: a list arrives as prose with its own markers in it.
#[test]
fn a_list_arrives_as_a_paragraph_with_its_markers_intact() {
    assert_eq!(
        blocks("- one\n- two\n"),
        vec![Block::Paragraph {
            text: "- one - two".into()
        }]
    );
}

#[test]
fn an_empty_readme_has_no_blocks() {
    assert!(blocks("").is_empty());
    assert!(blocks("\n\n   \n").is_empty());
}
