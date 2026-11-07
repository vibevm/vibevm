//! Optional non-validating XML event summary; exact raw bytes always survive.
//!
//! UTF-8 XML 1.0 only. No DTD/entity resolution, PI interpretation, namespaces,
//! complete XML Name/PI-target grammar or XML validity proof. Even malformed
//! lexical names accepted by upstream may yield a summary. This is not the Vibe reader.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION");
mod inspect;
/// Caller-selected structural work budgets, independent of raw retrieval.
/// None means explicitly unbounded on that axis. There is no implicit default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectionLimits {
    pub max_input_bytes: Option<usize>,
    pub max_depth: Option<usize>,
    pub max_elements: Option<usize>,
    /// Raw text/CDATA payload bytes plus reference spellings; not decoded text.
    pub max_text_bytes: Option<usize>,
    /// Non-EOF events, including comments and declarations.
    pub max_events: Option<usize>,
}
impl InspectionLimits {
    /// Explicitly request no structural quotas. No value here caps raw access.
    /// ```
    /// use vibe_specdoc::xml_data::InspectionLimits;
    /// let limits = InspectionLimits { max_depth: Some(8), ..InspectionLimits::unlimited() };
    /// assert_eq!(limits.max_input_bytes, None);
    /// ```
    pub const fn unlimited() -> Self {
        Self {
            max_input_bytes: None,
            max_depth: None,
            max_elements: None,
            max_text_bytes: None,
            max_events: None,
        }
    }
}
/// Which optional work budget was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionBudget {
    InputBytes,
    Depth,
    Elements,
    TextBytes,
    Events,
}
/// A lexical event summary, not a DOM or certificate of valid XML/namespaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlOutline {
    pub root: String,
    pub elements: usize,
    pub max_depth: usize,
    /// Raw event payload/reference spelling bytes, including outside whitespace.
    /// Excludes attributes, markup delimiters, comments and the BOM.
    pub text_bytes: usize,
    pub comments: usize,
    /// Non-declaration PI events, retained as inert syntax only.
    pub processing_instructions: usize,
}
/// Fixed structural refusal; no source text, filesystem or private config leaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlDataIssue {
    InvalidUtf8,
    UnsupportedEncoding,
    UnsupportedXmlVersion,
    MalformedEvents,
    InvalidDeclaration,
    ForbiddenDtd,
    ForbiddenReference,
    LimitExceeded {
        budget: InspectionBudget,
        limit: usize,
    },
    CounterOverflow,
}
impl std::fmt::Display for XmlDataIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION: optional XML inspection {self:?}; fix surface: supplied XML bytes or explicit inspection budgets; original raw bytes remain available"
        )
    }
}
impl std::error::Error for XmlDataIssue {}
/// Optional structure is orthogonal to preserved bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataInspection {
    NotRequested,
    Available(XmlOutline),
    Unavailable(XmlDataIssue),
}
/// Borrowed original bytes, including invalid encodings, BOM and line endings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservedData<'a> {
    raw: &'a [u8],
    inspection: DataInspection,
}
impl<'a> PreservedData<'a> {
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }
    pub fn inspection(&self) -> &DataInspection {
        &self.inspection
    }
}
/// Preserve opaque content without decoding or attempting structure.
/// ```
/// use vibe_specdoc::xml_data::{preserve_opaque, DataInspection};
/// let bytes = b"\xff\0<not-XML";
/// let view = preserve_opaque(bytes);
/// assert!(std::ptr::eq(view.raw(), bytes));
/// assert_eq!(view.inspection(), &DataInspection::NotRequested);
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION",
    r = 1
)]
pub fn preserve_opaque(raw: &[u8]) -> PreservedData<'_> {
    PreservedData {
        raw,
        inspection: DataInspection::NotRequested,
    }
}
/// Summarize caller-supplied UTF-8 XML 1.0 event structure with explicit budgets.
/// This does not validate complete XML Name/QName/PI-target grammar or namespaces; even
/// Available is not a well-formedness certificate. PIs are counted as inert events.
/// Budgets bound selected inspection counters, not elapsed time or raw retrieval.
/// ```
/// use vibe_specdoc::xml_data::{inspect_xml_data, InspectionLimits, DataInspection, XmlDataIssue};
/// let raw = b"<r><item/></r>\r\n";
/// let view = inspect_xml_data(raw, InspectionLimits::unlimited());
/// assert!(matches!(view.inspection(), DataInspection::Available(outline) if outline.elements==2));
/// let refused = inspect_xml_data(raw, InspectionLimits { max_depth: Some(1), ..InspectionLimits::unlimited() });
/// assert!(matches!(refused.inspection(), DataInspection::Unavailable(XmlDataIssue::LimitExceeded{..})));
/// assert!(std::ptr::eq(refused.raw(), raw));
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION",
    r = 1
)]
pub fn inspect_xml_data(raw: &[u8], limits: InspectionLimits) -> PreservedData<'_> {
    let inspection = inspect::inspect(raw, limits)
        .map_or_else(DataInspection::Unavailable, DataInspection::Available);
    PreservedData { raw, inspection }
}
#[cfg(test)]
mod tests;
