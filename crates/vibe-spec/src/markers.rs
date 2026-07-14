//! Reversible emission markers (PROP-035 §11).
//!
//! When the compiler places a block into `STATIC.md`, it wraps it in an
//! open **and** close comment carrying the block's full `spec://` key — which
//! already encodes package (`group/name`), document (`doc-path`), and section
//! (`anchor`). The paired markers make the compiled document **reversible**:
//! [`decompile`] splits it straight back into its blocks, the same
//! bidirectional traceability specmap gives code. `#embed` splices carry their
//! own nested `<!-- embed: … -->` markers (see [`crate::expand_embeds`]); those
//! stay inside a block's body here.

/// The open marker for a compiled block keyed by its `spec://` address.
pub fn open(key: &str) -> String {
    format!("<!-- vibe:begin {key} -->")
}

/// The close marker matching [`open`].
pub fn close(key: &str) -> String {
    format!("<!-- vibe:end {key} -->")
}

/// One top-level block recovered from a compiled document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub key: String,
    pub body: String,
}

/// Split a compiled document back into its top-level blocks by their markers,
/// proving the emission is reversible (§11). Nested markers (an `#embed`'s) stay
/// in the enclosing block's body.
pub fn decompile(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;

    for line in text.lines() {
        if let Some(key) = parse_open(line) {
            current = Some((key, Vec::new()));
        } else if parse_close(line).is_some() {
            if let Some((key, body)) = current.take() {
                blocks.push(Block {
                    key,
                    body: body.join("\n"),
                });
            }
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line);
        }
    }
    blocks
}

fn parse_open(line: &str) -> Option<String> {
    line.trim()
        .strip_prefix("<!-- vibe:begin ")
        .and_then(|rest| rest.strip_suffix(" -->"))
        .map(str::to_string)
}

fn parse_close(line: &str) -> Option<String> {
    line.trim()
        .strip_prefix("<!-- vibe:end ")
        .and_then(|rest| rest.strip_suffix(" -->"))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_and_close_are_paired() {
        assert_eq!(
            open("spec://vibevm/a#r"),
            "<!-- vibe:begin spec://vibevm/a#r -->"
        );
        assert_eq!(
            close("spec://vibevm/a#r"),
            "<!-- vibe:end spec://vibevm/a#r -->"
        );
    }

    #[test]
    fn decompile_recovers_blocks() {
        let doc = format!(
            "{}\nbody line one\nbody line two\n{}\n",
            open("spec://vibevm/a#r"),
            close("spec://vibevm/a#r"),
        );
        let blocks = decompile(&doc);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].key, "spec://vibevm/a#r");
        assert_eq!(blocks[0].body, "body line one\nbody line two");
    }

    #[test]
    fn two_blocks_recovered_in_order() {
        let doc = format!(
            "{}\nfirst\n{}\n{}\nsecond\n{}\n",
            open("spec://vibevm/a#r"),
            close("spec://vibevm/a#r"),
            open("spec://vibevm/b#r"),
            close("spec://vibevm/b#r"),
        );
        let keys: Vec<String> = decompile(&doc).into_iter().map(|b| b.key).collect();
        assert_eq!(keys, ["spec://vibevm/a#r", "spec://vibevm/b#r"]);
    }

    #[test]
    fn nested_embed_markers_stay_in_the_body() {
        let doc = format!(
            "{}\n<!-- embed: spec://vibevm/x#r -->\nembedded text\n<!-- /embed: spec://vibevm/x#r -->\n{}\n",
            open("spec://vibevm/a#r"),
            close("spec://vibevm/a#r"),
        );
        let blocks = decompile(&doc);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].body.contains("embedded text"));
        assert!(blocks[0].body.contains("<!-- embed:"));
    }
}
