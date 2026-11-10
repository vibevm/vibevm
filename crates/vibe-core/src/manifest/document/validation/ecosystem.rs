//! The one `[[application.ecosystem.recommends]]` rule that needs both
//! halves of the document (PROP-075 `#ECOSYSTEM-RECOMMENDS`). Each row's
//! own rules are `ApplicationDecl::validate_ecosystems`; this is the reach
//! check across the installer options, and it lives in its own cell for
//! the reason every other document rule family does.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS");

use crate::error::Result;
use crate::manifest::{FieldError, FieldPath};

use super::Manifest;

impl Manifest {
    /// The one `[[application.ecosystem.recommends]]` rule that needs
    /// both halves of the document: a recommendation `id` may not collide
    /// with an authored feature of **any** installer option, because the
    /// row synthesises a feature of that id into every one of them
    /// (PROP-075 `#ECOSYSTEM-RECOMMENDS`). The reserved shape is the other
    /// way round: an author who wants a tree, a requirement or an
    /// exclusive group writes the feature rows and refers to the item with
    /// a recommends row of the same id — which is admitted, and is the one
    /// way the two spellings meet.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS",
        r = 1
    )]
    pub(super) fn validate_ecosystem_reach(&self) -> Result<()> {
        let Some(application) = &self.application else {
            return Ok(());
        };
        if application.ecosystem.is_empty() {
            return Ok(());
        }
        let at = FieldPath::root().table("application").table("ecosystem");
        for (position, row) in application.ecosystem.iter().enumerate() {
            for (index, recommend) in row.recommends.iter().enumerate() {
                let Some((option, place)) =
                    self.installer_options
                        .iter()
                        .enumerate()
                        .find_map(|(place, option)| {
                            option
                                .features
                                .iter()
                                .any(|feature| {
                                    feature.origin.is_authored() && feature.id == recommend.id
                                })
                                .then_some((option, place))
                        })
                else {
                    continue;
                };
                let authored = FieldPath::root()
                    .table("installer_options")
                    .index(place)
                    .table("features");
                let here = at
                    .index(position)
                    .table("recommends")
                    .index(index)
                    .field("id");
                return Err(FieldError::new(
                    here.clone(),
                    format!(
                        "{here}: recommendation id `{}` collides with the authored feature of                          that id in `{authored}` (installer option `{}`); the row synthesises a                          feature of its own id into every option, so refer to the authored row                          with a recommends row of the same id instead of declaring both",
                        recommend.id, option.id
                    ),
                )
                .into());
            }
        }
        Ok(())
    }
}
