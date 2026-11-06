//! Ephemeral positions recorded by the existing dialect writer, not reparsed
//! from marker-looking prose. Ordinary writers leave capture disabled.

use quick_xml::Writer;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub(crate) struct TextSite {
    pub(crate) before: usize,
    pub(crate) text: usize,
    pub(crate) after: usize,
    pub(crate) value: String,
}

#[derive(Debug, Clone)]
pub(crate) struct HeadingSite {
    pub(crate) before: usize,
    pub(crate) after: usize,
}

pub(crate) struct Output {
    inner: Writer<Vec<u8>>,
    pub(crate) capture: bool,
    pub(crate) texts: Vec<TextSite>,
    pub(crate) headings: Vec<HeadingSite>,
}

impl Output {
    pub(crate) fn new(capture: bool) -> Self {
        Self {
            inner: Writer::new(Vec::new()),
            capture,
            texts: Vec::new(),
            headings: Vec::new(),
        }
    }
    pub(crate) fn position(&self) -> usize {
        self.inner.get_ref().len()
    }
    pub(crate) fn record_text(&mut self, before: usize, text: usize, value: &str) {
        if self.capture {
            self.texts.push(TextSite {
                before,
                text,
                after: self.position(),
                value: value.to_owned(),
            });
        }
    }
    pub(crate) fn record_heading(&mut self, before: usize) {
        if self.capture {
            self.headings.push(HeadingSite {
                before,
                after: self.position(),
            });
        }
    }
    pub(crate) fn finish(self) -> (Vec<u8>, Vec<TextSite>, Vec<HeadingSite>) {
        (self.inner.into_inner(), self.texts, self.headings)
    }
}
impl Deref for Output {
    type Target = Writer<Vec<u8>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for Output {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
