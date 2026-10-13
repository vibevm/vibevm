//! Authored cooperative source-effect contract, never a phase/name inference.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DECLARED-SOURCE-EFFECTS");
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeclaredEffect {
    BuildArtifact,
    EvidenceObserver,
    ProductSource,
    ReadOnly,
}
impl DeclaredEffect {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BuildArtifact => "build-artifact",
            Self::EvidenceObserver => "evidence-observer",
            Self::ProductSource => "product-source",
            Self::ReadOnly => "read-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<DeclaredEffect>", into = "Vec<DeclaredEffect>")]
pub struct DeclaredEffects(Vec<DeclaredEffect>);
impl DeclaredEffects {
    pub fn new(mut effects: Vec<DeclaredEffect>) -> Result<Self, String> {
        if effects.is_empty() {
            return Err("declared effects must be nonempty; absence is unclassified".into());
        }
        effects.sort();
        if effects.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("duplicate declared effect".into());
        }
        Ok(Self(effects))
    }
    pub fn contains(&self, effect: DeclaredEffect) -> bool {
        self.0.contains(&effect)
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = DeclaredEffect> + '_ {
        self.0.iter().copied()
    }
}
impl TryFrom<Vec<DeclaredEffect>> for DeclaredEffects {
    type Error = String;
    fn try_from(value: Vec<DeclaredEffect>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<DeclaredEffects> for Vec<DeclaredEffect> {
    fn from(value: DeclaredEffects) -> Self {
        value.0
    }
}

#[cfg(test)]
mod m_03_a_1_cases {
    use super::*;
    use crate::manifest::Manifest;
    fn text(effects: &str) -> String {
        format!(
            "[project]\nname='demo'\nversion='1.0.0'\n[[extension]]\nid='compile'\npoint='phase:build'\nhandler={{kind='builtin',name='log'}}\n{effects}\n"
        )
    }
    #[test]
    fn actual_authored_effects_are_closed_canonical_and_never_default_safe() {
        let legacy = Manifest::parse_str(&text("")).unwrap();
        assert_eq!(legacy.extensions[0].effects, None);
        let current = Manifest::parse_str(&text(
            "effects=['read-only','product-source','build-artifact']",
        ))
        .unwrap();
        let effects = current.extensions[0].effects.as_ref().unwrap();
        assert!(effects.contains(DeclaredEffect::ProductSource));
        assert_eq!(
            effects
                .iter()
                .map(DeclaredEffect::as_str)
                .collect::<Vec<_>>(),
            ["build-artifact", "product-source", "read-only"]
        );
        for (input, diagnostic) in [
            ("effects=[]", "nonempty"),
            ("effects=['read-only','read-only']", "duplicate"),
            ("effects=['safe']", "unknown variant"),
        ] {
            let error = Manifest::parse_str(&text(input)).unwrap_err().to_string();
            assert!(error.contains(diagnostic), "{error}");
        }
    }
}
