//! Honest source-format conversion classification for PROP-051.
//!
//! Conversion always projects the emitted target back into the source form.
//! Byte differences are reported only after the pivot IR is proven stable;
//! any IR drift is a hard refusal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-051#convert-source");

use crate::doc::SpecDoc;
use crate::{Result, from_markdown, from_xml, to_markdown, to_xml};

/// Target serialisation for a conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Markdown source to dialect XML.
    ToXml,
    /// Dialect XML source to Markdown.
    ToMarkdown,
}

/// Result of the reverse-projection honesty check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conversion {
    /// Reverse projection reproduced the source byte-for-byte.
    ByteStable { output: String },
    /// The IR survived, but source spelling or unmodelled content did not.
    IrStableLoss { output: String, loss: String },
    /// The target round-trip changed the pivot IR.
    IrDivergent { detail: String },
}

/// Classify a conversion according to PROP-051 ##HONESTY-BY-REVERSE.
///
/// An error means the supplied source does not parse in the selected source
/// format. Failures after that point are reported as [`Conversion::IrDivergent`]
/// so callers can distinguish a bad input from a broken pivot round-trip.
pub fn convert(source: &str, direction: Direction) -> Result<Conversion> {
    match direction {
        Direction::ToXml => convert_markdown(source),
        Direction::ToMarkdown => convert_xml(source),
    }
}

fn convert_markdown(source: &str) -> Result<Conversion> {
    let ir = from_markdown(source)?;
    let output = to_xml(&ir);
    let back_ir = match from_xml(&output) {
        Ok(doc) => doc,
        Err(error) => {
            return Ok(Conversion::IrDivergent {
                detail: format!("emitted XML did not parse: {error}"),
            });
        }
    };
    let back = to_markdown(&back_ir);
    Ok(classify_projection(
        source,
        &back,
        &ir,
        &back_ir,
        output,
        from_markdown,
    ))
}

fn convert_xml(source: &str) -> Result<Conversion> {
    let ir = from_xml(source)?;
    let output = to_markdown(&ir);
    let back_ir = match from_markdown(&output) {
        Ok(doc) => doc,
        Err(error) => {
            return Ok(Conversion::IrDivergent {
                detail: format!("emitted Markdown did not parse: {error}"),
            });
        }
    };
    let back = to_xml(&back_ir);
    Ok(classify_projection(
        source, &back, &ir, &back_ir, output, from_xml,
    ))
}

/// Pure seam for the class-3 proof: tests can supply genuinely distinct IRs
/// without depending on an emitter defect being present in production.
pub(crate) fn classify_projection(
    source: &str,
    back: &str,
    ir: &SpecDoc,
    back_ir: &SpecDoc,
    output: String,
    parse_back: fn(&str) -> Result<SpecDoc>,
) -> Conversion {
    if ir != back_ir {
        return Conversion::IrDivergent {
            detail: "target round-trip changed the pivot IR".to_string(),
        };
    }
    if source == back {
        return Conversion::ByteStable { output };
    }
    match parse_back(back) {
        Ok(projected_ir) if projected_ir == *ir => Conversion::IrStableLoss {
            output,
            loss: unified_diff(source, back),
        },
        Ok(_) => Conversion::IrDivergent {
            detail: "reverse projection reparsed to a different pivot IR".to_string(),
        },
        Err(error) => Conversion::IrDivergent {
            detail: format!("reverse projection did not parse: {error}"),
        },
    }
}

#[derive(Clone, Copy)]
enum Edit<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

/// A compact unified line diff with two unchanged context lines per hunk.
fn unified_diff(source: &str, back: &str) -> String {
    let old: Vec<&str> = source.split('\n').collect();
    let new: Vec<&str> = back.split('\n').collect();
    let edits = lcs_edits(&old, &new);
    let changes: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter_map(|(index, edit)| match edit {
            Edit::Equal(_) => None,
            Edit::Delete(_) | Edit::Insert(_) => Some(index),
        })
        .collect();

    let mut out = String::from("--- source\n+++ reverse-projection\n");
    let mut change = 0;
    while change < changes.len() {
        let first = changes[change];
        let mut last = first;
        while change + 1 < changes.len() && changes[change + 1] <= last + 5 {
            change += 1;
            last = changes[change];
        }
        let start = context_start(&edits, first, 2);
        let end = context_end(&edits, last, 2);
        write_hunk(&mut out, &edits, start, end);
        change += 1;
    }
    out
}

fn lcs_edits<'a>(old: &'a [&'a str], new: &'a [&'a str]) -> Vec<Edit<'a>> {
    let mut lengths = vec![vec![0_usize; new.len() + 1]; old.len() + 1];
    for old_index in (0..old.len()).rev() {
        for new_index in (0..new.len()).rev() {
            lengths[old_index][new_index] = if old[old_index] == new[new_index] {
                lengths[old_index + 1][new_index + 1] + 1
            } else {
                lengths[old_index + 1][new_index].max(lengths[old_index][new_index + 1])
            };
        }
    }

    let mut edits = Vec::new();
    let (mut old_index, mut new_index) = (0, 0);
    while old_index < old.len() && new_index < new.len() {
        if old[old_index] == new[new_index] {
            edits.push(Edit::Equal(old[old_index]));
            old_index += 1;
            new_index += 1;
        } else if lengths[old_index + 1][new_index] >= lengths[old_index][new_index + 1] {
            edits.push(Edit::Delete(old[old_index]));
            old_index += 1;
        } else {
            edits.push(Edit::Insert(new[new_index]));
            new_index += 1;
        }
    }
    edits.extend(old[old_index..].iter().map(|line| Edit::Delete(line)));
    edits.extend(new[new_index..].iter().map(|line| Edit::Insert(line)));
    edits
}

fn context_start(edits: &[Edit<'_>], first: usize, context: usize) -> usize {
    let mut start = first;
    let mut remaining = context;
    while start > 0 && remaining > 0 {
        start -= 1;
        if matches!(edits[start], Edit::Equal(_)) {
            remaining -= 1;
        }
    }
    start
}

fn context_end(edits: &[Edit<'_>], last: usize, context: usize) -> usize {
    let mut end = last + 1;
    let mut remaining = context;
    while end < edits.len() && remaining > 0 {
        if matches!(edits[end], Edit::Equal(_)) {
            remaining -= 1;
        }
        end += 1;
    }
    end
}

fn write_hunk(out: &mut String, edits: &[Edit<'_>], start: usize, end: usize) {
    let old_start = 1 + edits[..start]
        .iter()
        .filter(|edit| !matches!(edit, Edit::Insert(_)))
        .count();
    let new_start = 1 + edits[..start]
        .iter()
        .filter(|edit| !matches!(edit, Edit::Delete(_)))
        .count();
    let old_count = edits[start..end]
        .iter()
        .filter(|edit| !matches!(edit, Edit::Insert(_)))
        .count();
    let new_count = edits[start..end]
        .iter()
        .filter(|edit| !matches!(edit, Edit::Delete(_)))
        .count();
    out.push_str(&format!(
        "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"
    ));
    for edit in &edits[start..end] {
        match edit {
            Edit::Equal(line) => out.push_str(&format!(" {line}\n")),
            Edit::Delete(line) => out.push_str(&format!("-{line}\n")),
            Edit::Insert(line) => out.push_str(&format!("+{line}\n")),
        }
    }
}
