//! Physical comment locations from the same quick-xml token grammar used by
//! the incumbent reader. This adapter does not validate a SpecDoc or infer
//! generated meaning; a consumer still decodes its own reserved payloads.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#XML-COMMENT-TOKEN-INVENTORY"
);

use quick_xml::{Reader, events::Event};

/// One actual XML comment in a possibly multi-document generated tape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlCommentSpan {
    pub start: usize,
    pub end: usize,
    pub depth: usize,
}

/// A lexical refusal, with context needed to locate an unfinished outer frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlCommentScanError {
    pub offset: usize,
    pub depth: usize,
    pub unclosed_comment: bool,
    pub message: String,
}

impl std::fmt::Display for XmlCommentScanError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "XML comment scan refused at byte {}: {}",
            self.offset, self.message
        )
    }
}
impl std::error::Error for XmlCommentScanError {}

/// Iterate actual comment tokens, excluding CDATA, escaped text and attributes.
/// Physical offsets are UTF-8 byte positions and depth is the open-element
/// count. This is token observation, not a well-formed-document certificate.
/// ```
/// let xml = "<p><![CDATA[<!-- literal -->]]><!-- actual --></p>";
/// let comments = vibe_specdoc::xml_comment_spans(xml).collect::<Result<Vec<_>,_>>().unwrap();
/// assert_eq!(comments.len(),1);
/// assert_eq!(&xml[comments[0].start..comments[0].end],"<!-- actual -->");
/// assert_eq!(comments[0].depth,1);
/// ```
pub fn xml_comment_spans(
    xml: &str,
) -> impl Iterator<Item = Result<XmlCommentSpan, XmlCommentScanError>> + '_ {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_comments = true;
    reader.config_mut().check_end_names = true;
    Comments {
        reader,
        depth: 0,
        done: false,
    }
}

struct Comments<'a> {
    reader: Reader<&'a [u8]>,
    depth: usize,
    done: bool,
}

impl Iterator for Comments<'_> {
    type Item = Result<XmlCommentSpan, XmlCommentScanError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let start = self.reader.buffer_position() as usize;
            match self.reader.read_event() {
                Ok(Event::Start(_)) => self.depth += 1,
                Ok(Event::End(_)) => self.depth = self.depth.saturating_sub(1),
                Ok(Event::Comment(_)) => {
                    return Some(Ok(XmlCommentSpan {
                        start,
                        end: self.reader.buffer_position() as usize,
                        depth: self.depth,
                    }));
                }
                Ok(Event::Eof) => {
                    self.done = true;
                    return None;
                }
                Ok(_) => {}
                Err(error) => {
                    self.done = true;
                    return Some(Err(XmlCommentScanError {
                        offset: self.reader.error_position() as usize,
                        depth: self.depth,
                        unclosed_comment: matches!(
                            error,
                            quick_xml::Error::Syntax(
                                quick_xml::errors::SyntaxError::UnclosedComment
                            )
                        ),
                        message: error.to_string(),
                    }));
                }
            }
        }
    }
}
