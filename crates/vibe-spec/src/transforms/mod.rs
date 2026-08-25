//! Pure compiler transforms, before manifest activation or pass scheduling.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED");

mod xml_minify;

pub use xml_minify::{XmlMinifyError, minify_emitted_xml};
