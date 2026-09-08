//! Registered source-format identities at the compiler's parse seat.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-FRONTEND");

use std::collections::BTreeMap;

use crate::compiler::ir::{DocumentIr, SourceFormatId, SourceIr};
use crate::compiler::pass::IrPayload;

use super::catalog::{CatalogPass, PassCatalogCompileError, PassCatalogError};
use super::plan::PassEntry;

const BUILTIN_FORMATS: [&str; 3] = ["markdown", "md", "xml"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrontendCatalog {
    formats: BTreeMap<String, CatalogPass>,
}

impl FrontendCatalog {
    pub(super) fn register(&mut self, entry: &PassEntry) -> Result<(), PassCatalogError> {
        let formats = entry
            .declaration()
            .formats
            .as_ref()
            .filter(|formats| !formats.is_empty())
            .ok_or_else(|| PassCatalogError::MissingFormats {
                key: entry.key().clone(),
            })?;
        let binding = CatalogPass::new(entry, SourceIr::SHAPE, DocumentIr::SHAPE)?;
        for authored in formats {
            let format = SourceFormatId::new(authored.clone()).map_err(|_| {
                PassCatalogError::InvalidFormat {
                    key: entry.key().clone(),
                    format: bounded(authored),
                }
            })?;
            if !is_physical_extension(format.as_str()) {
                return Err(PassCatalogError::InvalidFormat {
                    key: entry.key().clone(),
                    format: bounded(authored),
                });
            }
            if BUILTIN_FORMATS.contains(&format.as_str()) {
                return Err(PassCatalogError::BuiltinFormat {
                    key: entry.key().clone(),
                    format: format.as_str().to_owned(),
                });
            }
            if let Some(first) = self.formats.get(format.as_str()) {
                return Err(PassCatalogError::DuplicateFormat {
                    format: format.as_str().to_owned(),
                    first: first.key().clone(),
                    second: entry.key().clone(),
                });
            }
            self.formats
                .insert(format.as_str().to_owned(), binding.clone());
        }
        Ok(())
    }

    pub(crate) fn get(&self, format: &str) -> Option<&CatalogPass> {
        self.formats.get(format)
    }

    pub(crate) fn len(&self) -> usize {
        self.formats.len()
    }

    pub(crate) fn physical_formats(&self) -> impl Iterator<Item = &str> {
        self.formats.keys().map(String::as_str)
    }

    pub(crate) fn deferred(
        &self,
        format: &str,
        physical_stem: &str,
    ) -> Option<PassCatalogCompileError> {
        self.formats.get(format).map(|pass| {
            PassCatalogError::FrontendDeferred {
                key: pass.key().clone(),
                format: format.to_owned(),
                physical_stem: physical_stem.to_owned(),
            }
            .into()
        })
    }
}

fn is_physical_extension(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn bounded(value: &str) -> String {
    let mut characters = value.chars();
    let mut result = characters.by_ref().take(80).collect::<String>();
    if characters.next().is_some() {
        result.push('…');
    }
    result
}
