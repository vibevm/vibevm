//! Which protocol an index document *is*, and which one the writer writes
//! (`PROP-059#index`, `#proxy-index-upgrade`).
//!
//! Reading admits the closed set `/3`…`/8`, promoting each older generation
//! through the typed `upgrade*` modules; writing emits **the lowest protocol
//! that carries what the home actually retains**, which is the whole reason
//! a home that uses none of a generation's facts keeps its old format and
//! every existing PATH proxy goes on reading it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy-index-upgrade");

use super::*;

impl Index {
    /// Read the exact protocol's closed schema; loading never writes a migration.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        Self::parse_with_protocol(bytes).map(|(index, _)| index)
    }

    pub(super) fn parse_with_protocol(bytes: &[u8]) -> Result<(Self, u8), String> {
        #[derive(serde::Deserialize)]
        struct Header {
            protocol: String,
        }
        let header: Header = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let document = match header.protocol.as_str() {
            LEGACY_PROTOCOL => upgrade_seven::promote(upgrade_six::promote(upgrade_five::promote(
                upgrade_four::promote(upgrade::promote(
                    serde_json::from_slice::<
                        vibe_wire::generated::application::e2::index::ApplicationIndex,
                    >(bytes)
                    .map_err(|e| e.to_string())?,
                )),
            ))),
            FOUR_PROTOCOL => upgrade_seven::promote(upgrade_six::promote(upgrade_five::promote(
                upgrade_four::promote(serde_json::from_slice(bytes).map_err(|e| e.to_string())?),
            ))),
            EVIDENCE_PROTOCOL => upgrade_seven::promote(upgrade_six::promote(
                upgrade_five::promote(serde_json::from_slice(bytes).map_err(|e| e.to_string())?),
            )),
            PATHS_PROTOCOL => upgrade_seven::promote(upgrade_six::promote(
                serde_json::from_slice(bytes).map_err(|e| e.to_string())?,
            )),
            WEAK_PROTOCOL => {
                upgrade_seven::promote(serde_json::from_slice(bytes).map_err(|e| e.to_string())?)
            }
            INDEX_PROTOCOL => serde_json::from_slice(bytes).map_err(|e| e.to_string())?,
            _ => return Err("unsupported application index protocol".into()),
        };
        Self::from_wire(document).map(|index| {
            (
                index,
                match header.protocol.as_str() {
                    INDEX_PROTOCOL => 8,
                    WEAK_PROTOCOL => 7,
                    PATHS_PROTOCOL => 6,
                    EVIDENCE_PROTOCOL => 5,
                    FOUR_PROTOCOL => 4,
                    _ => 3,
                },
            )
        })
    }

    /// Whether any retained data needs the current closed index4 reader.
    pub(crate) fn requires_reader_four(&self) -> bool {
        self.installations.values().any(|r| {
            r.configuration.is_some() || r.pending.is_some() || !r.secret_origins.is_empty()
        })
    }

    /// Whether any retained data needs the current closed index6 reader:
    /// an exposed payload directory, a slot switched off, or a slot's
    /// applied link-directory PATH entries (`PROP-059#path-directories`,
    /// `#slot-off`).
    pub(crate) fn requires_reader_six(&self) -> bool {
        self.installations.values().any(|r| !r.path_dirs.is_empty())
            || self
                .slots
                .values()
                .any(|state| state.off || !state.path_env.is_empty())
    }

    /// Whether any retained data needs the current closed index7 reader:
    /// the template a rendered manifest was produced from, a declared
    /// migration entry, or a recorded migration outcome (`PROP-075`
    /// `#TEMPLATE-LOCK-RECORD`, `#MIGRATION-DECLARATION`,
    /// `#MIGRATION-FAILURE`).
    pub(crate) fn requires_reader_seven(&self) -> bool {
        self.installations
            .values()
            .any(|r| r.template.is_some() || !r.migrations.is_empty() || !r.migrated.is_empty())
    }

    /// Whether any retained data needs the current closed index8 reader: a
    /// weak requirer edge (`PROP-077#APP-RECOMMENDS-MECHANICS`), or an
    /// ecosystem adapter's record of what it last applied and last observed
    /// (`PROP-075#SLOT-EXTRAS`). Two phases' facts, one generation and one
    /// predicate — which is exactly why this reads as the ladder's rung and
    /// not as the weak edge's own question.
    ///
    /// A record that retains nothing does not count, and is not written
    /// either: an adapter that only ever skipped must not push a home onto
    /// a reader its PATH proxies may not have.
    pub(crate) fn requires_reader_eight(&self) -> bool {
        self.installations.values().any(|r| {
            r.lifecycle.requires.iter().any(|edge| edge.weak)
                || r.extras.iter().any(|record| !record.is_empty())
        })
    }

    pub(crate) fn minimum_reader(&self) -> u8 {
        if self.requires_reader_eight() {
            8
        } else if self.requires_reader_seven() {
            7
        } else if self.requires_reader_six() {
            6
        } else if self
            .installations
            .values()
            .any(|r| r.source_evidence.is_some())
        {
            5
        } else if self.requires_reader_four() {
            4
        } else {
            3
        }
    }
}
