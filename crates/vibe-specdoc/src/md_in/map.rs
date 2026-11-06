//! Source offsets collected at the same scalar extraction sites as the pivot.

use std::cell::RefCell;

#[derive(Debug, Clone)]
pub(crate) struct TextSource {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) value: String,
    /// Original source offset for each emitted UTF-8 byte, then the final gap.
    pub(crate) bytes: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct HeadingSource {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Default)]
pub(super) struct Capture {
    pub(super) texts: RefCell<Vec<TextSource>>,
    pub(super) headings: RefCell<Vec<HeadingSource>>,
}

pub(crate) struct SourceMap {
    pub(crate) texts: Vec<TextSource>,
    pub(crate) headings: Vec<HeadingSource>,
}

impl Capture {
    pub(super) fn finish(self) -> SourceMap {
        SourceMap {
            texts: self.texts.into_inner(),
            headings: self.headings.into_inner(),
        }
    }
}

impl super::Adapter<'_> {
    pub(super) fn line_offset(&self, one_based: usize) -> usize {
        self.lines
            .iter()
            .take(one_based.saturating_sub(1))
            .map(|line| line.len() + 1)
            .sum()
    }

    pub(super) fn record_plain(
        &self,
        block: &progress_core::doc::Block,
        start: usize,
        end: usize,
        trimmed: &str,
    ) {
        if let Some(capture) = &self.capture {
            let base = self.line_offset(block.line_start);
            let offset = trimmed.as_ptr() as usize - block.source_text.as_ptr() as usize;
            capture.texts.borrow_mut().push(TextSource {
                start: base + start,
                end: base + end,
                bytes: (base + offset..=base + offset + trimmed.len()).collect(),
                value: trimmed.to_owned(),
            });
        }
    }

    pub(super) fn record_fact(
        &self,
        b: &progress_core::doc::Block,
        f: &progress_core::doc::Fact,
        effective: &str,
        bare: &str,
    ) {
        use progress_core::doc::FactKind;
        use progress_core::parse::blockquote_prefix_len;
        let body = &b.source_text[f.span.0..f.span.1];
        if let Some(capture) = &self.capture {
            let start = self.line_offset(b.line_start) + f.span.0;
            let end = self.line_offset(b.line_start) + f.span.1;
            let quoted = (f.kind == FactKind::Para || f.kind == FactKind::Lead)
                && body.lines().all(|line| line.trim_start().starts_with('>'));
            let mut origins = Vec::new();
            if quoted {
                let mut offset = start;
                for (index, line) in body.lines().enumerate() {
                    if index > 0 {
                        origins.push(offset.saturating_sub(1));
                    }
                    let indent = line.len() - line.trim_start().len();
                    let prefix = indent + blockquote_prefix_len(&line[indent..]);
                    origins.extend(offset + prefix..offset + line.len());
                    offset += line.len() + 1;
                }
                origins.push(end);
            } else {
                origins.extend(start..=end);
            }
            let trimmed = bare.trim();
            let offset = trimmed.as_ptr() as usize - effective.as_ptr() as usize;
            capture.texts.borrow_mut().push(TextSource {
                start,
                end,
                value: trimmed.to_owned(),
                bytes: origins[offset..=offset + trimmed.len()].to_vec(),
            });
        }
    }
}
