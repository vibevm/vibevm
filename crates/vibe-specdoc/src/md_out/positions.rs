//! Optional positions captured by the incumbent Markdown writer.
use std::ops::{Deref, DerefMut};

pub(crate) struct TextSite {
    pub before: usize,
    pub after: usize,
    pub value: String,
    pub bytes: Vec<usize>,
    pub fenced: bool,
}
pub(crate) struct HeadingSite {
    pub before: usize,
    pub after: usize,
    pub value: String,
    pub bytes: Vec<usize>,
}
#[derive(Clone, Copy)]
pub(crate) struct Bounds {
    pub before: usize,
    pub after: usize,
}
pub(crate) struct Output {
    pub text: String,
    pub capture: bool,
    pub texts: Vec<TextSite>,
    pub headings: Vec<HeadingSite>,
    pub blocks: Vec<Bounds>,
    pub rows: Vec<Bounds>,
    pub statuses: Vec<Bounds>,
}
impl Output {
    pub fn new(capture: bool) -> Self {
        Self {
            text: String::new(),
            capture,
            texts: Vec::new(),
            headings: Vec::new(),
            blocks: Vec::new(),
            rows: Vec::new(),
            statuses: Vec::new(),
        }
    }
    pub fn record_text(&mut self, before: usize, text_start: usize, value: &str, fenced: bool) {
        if self.capture {
            self.texts.push(TextSite {
                before,
                after: self.text.len(),
                value: value.into(),
                bytes: (text_start..=text_start + value.len()).collect(),
                fenced,
            });
        }
    }
    pub fn record_heading(&mut self, before: usize, text_start: usize, value: &str) {
        if self.capture {
            self.headings.push(HeadingSite {
                before,
                after: self.text.len(),
                value: value.into(),
                bytes: (text_start..=text_start + value.len()).collect(),
            });
        }
    }
    pub fn record_status(&mut self, before: usize) {
        if self.capture {
            self.statuses.push(Bounds {
                before,
                after: self.len(),
            });
        }
    }
    pub fn record_row(&mut self, before: usize) {
        if self.capture {
            self.rows.push(Bounds {
                before,
                after: self.len(),
            });
        }
    }
    pub fn record_block(&mut self, before: usize) {
        if self.capture {
            self.blocks.push(Bounds {
                before,
                after: self.len(),
            });
        }
    }
    pub fn append(&mut self, mut inner: Self) {
        let base = self.text.len();
        inner.remap(|position| base + position);
        self.text.push_str(&inner.text);
        self.texts.extend(inner.texts);
        self.headings.extend(inner.headings);
        self.blocks.extend(inner.blocks);
        self.rows.extend(inner.rows);
        self.statuses.extend(inner.statuses);
    }
    pub fn append_quoted(&mut self, mut inner: Self) {
        let mut positions = self.capture.then(|| vec![0; inner.text.len() + 1]);
        let mut source = 0;
        for (index, line) in inner.text.lines().enumerate() {
            if index > 0 {
                if let Some(map) = &mut positions {
                    map[source - 1] = self.text.len();
                    if source >= 2 && inner.text.as_bytes()[source - 2] == b'\r' {
                        map[source - 2] = self.text.len();
                    }
                }
                self.text.push('\n');
            }
            self.text.push('>');
            if !line.is_empty() {
                self.text.push(' ');
            }
            if let Some(map) = &mut positions {
                for offset in 0..=line.len() {
                    map[source + offset] = self.text.len() + offset;
                }
            }
            self.text.push_str(line);
            source += line.len();
            if inner.text.as_bytes().get(source) == Some(&b'\r') {
                source += 1;
            }
            if inner.text.as_bytes().get(source) == Some(&b'\n') {
                source += 1;
            }
        }
        if let Some(mut positions) = positions {
            positions[inner.text.len()] = self.text.len();
            inner.remap(|position| positions[position]);
            self.texts.extend(inner.texts);
            self.headings.extend(inner.headings);
            self.blocks.extend(inner.blocks);
            self.rows.extend(inner.rows);
            self.statuses.extend(inner.statuses);
        }
    }
    fn remap(&mut self, map: impl Fn(usize) -> usize) {
        for text in &mut self.texts {
            text.before = map(text.before);
            text.after = map(text.after);
            for position in &mut text.bytes {
                *position = map(*position);
            }
        }
        for heading in &mut self.headings {
            heading.before = map(heading.before);
            heading.after = map(heading.after);
            for byte in &mut heading.bytes {
                *byte = map(*byte);
            }
        }
        for bounds in self
            .blocks
            .iter_mut()
            .chain(&mut self.rows)
            .chain(&mut self.statuses)
        {
            bounds.before = map(bounds.before);
            bounds.after = map(bounds.after);
        }
    }
}
impl Deref for Output {
    type Target = String;
    fn deref(&self) -> &String {
        &self.text
    }
}
impl DerefMut for Output {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.text
    }
}
