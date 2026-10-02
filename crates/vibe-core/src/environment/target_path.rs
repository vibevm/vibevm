//! Target byte grammar and explicit lexical models, never host/native proof.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY");
use super::{EnvironmentError, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPathPolicy {
    LinuxBytes,
    /// POSIX byte components under macOS lexical rules. The storage owner
    /// still proves physical containment, aliases and actual filesystem case.
    MacosPosixBytes,
    WindowsPortableWin32Utf8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseComparison {
    ByteSensitive,
    AsciiInsensitive,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetComponent {
    policy: TargetPathPolicy,
    bytes: Vec<u8>,
}
impl TargetComponent {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY",
        r = 2
    )]
    pub fn new(policy: TargetPathPolicy, bytes: Vec<u8>) -> Result<Self, EnvironmentError> {
        if bytes.is_empty()
            || bytes == b"."
            || bytes == b".."
            || bytes.contains(&0)
            || bytes.contains(&b'/')
        {
            return Err(invalid(
                "target.component",
                "empty, traversal, NUL and slash components are invalid",
            ));
        }
        if policy == TargetPathPolicy::WindowsPortableWin32Utf8 {
            let text = std::str::from_utf8(&bytes).map_err(|_| {
                invalid(
                    "target.component",
                    "Windows policy requires explicit UTF8 admission",
                )
            })?;
            if crate::manifest::is_windows_unsafe_component(text) {
                return Err(invalid(
                    "target.component",
                    "component is outside the Windows portable-Win32 subset",
                ));
            }
        }
        Ok(Self { policy, bytes })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn policy(&self) -> TargetPathPolicy {
        self.policy
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRelativePath {
    policy: TargetPathPolicy,
    components: Vec<TargetComponent>,
}
impl TargetRelativePath {
    /// Empty sequence denotes the root in absolute-path construction, never an
    /// empty component; resource construction requires an actual relative name.
    pub fn new(
        policy: TargetPathPolicy,
        components: Vec<Vec<u8>>,
    ) -> Result<Self, EnvironmentError> {
        Ok(Self {
            policy,
            components: components
                .into_iter()
                .map(|c| TargetComponent::new(policy, c))
                .collect::<Result<_, _>>()?,
        })
    }
    pub fn components(&self) -> &[TargetComponent] {
        &self.components
    }
    pub fn policy(&self) -> TargetPathPolicy {
        self.policy
    }
    pub(super) fn key(&self, model: CaseComparison) -> Option<Vec<Vec<u8>>> {
        self.components
            .iter()
            .map(|c| byte_key(c.bytes(), model))
            .collect()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetAnchor {
    PosixRoot,
    WindowsDrive(u8),
    WindowsUnc { server: Vec<u8>, share: Vec<u8> },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsoluteTargetPath {
    anchor: TargetAnchor,
    relative: TargetRelativePath,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalRelation {
    Equal,
    Ancestor,
    Descendant,
    Separate,
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum AnchorKey {
    Posix,
    Drive(Vec<u8>),
    Unc(Vec<u8>, Vec<u8>),
}
impl AbsoluteTargetPath {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY",
        r = 2
    )]
    pub fn new(
        policy: TargetPathPolicy,
        anchor: TargetAnchor,
        components: Vec<Vec<u8>>,
    ) -> Result<Self, EnvironmentError> {
        match (&anchor, policy) {
            (
                TargetAnchor::PosixRoot,
                TargetPathPolicy::LinuxBytes | TargetPathPolicy::MacosPosixBytes,
            ) => {}
            (TargetAnchor::WindowsDrive(letter), TargetPathPolicy::WindowsPortableWin32Utf8)
                if letter.is_ascii_alphabetic() => {}
            (
                TargetAnchor::WindowsUnc { server, share },
                TargetPathPolicy::WindowsPortableWin32Utf8,
            ) => {
                TargetComponent::new(policy, server.clone())?;
                TargetComponent::new(policy, share.clone())?;
            }
            _ => {
                return Err(invalid(
                    "target.anchor",
                    "anchor does not match the declared target policy",
                ));
            }
        }
        Ok(Self {
            anchor,
            relative: TargetRelativePath::new(policy, components)?,
        })
    }
    pub fn anchor(&self) -> &TargetAnchor {
        &self.anchor
    }
    pub fn components(&self) -> &[TargetComponent] {
        self.relative.components()
    }
    pub fn policy(&self) -> TargetPathPolicy {
        self.relative.policy()
    }
    pub fn appended(&self, path: &TargetRelativePath) -> Result<Self, EnvironmentError> {
        if path.policy() != self.policy() {
            return Err(invalid(
                "target.path",
                "cannot append a different target policy",
            ));
        }
        let mut output = self.clone();
        output.relative.components.extend(path.components.clone());
        Ok(output)
    }
    pub fn appended_component(&self, bytes: Vec<u8>) -> Result<Self, EnvironmentError> {
        self.appended(&TargetRelativePath::new(self.policy(), vec![bytes])?)
    }
    /// A lexical relation only; aliases, native collation and physical
    /// containment/disjointness require the storage owner's actual admission.
    pub fn lexical_relation(&self, other: &Self, model: CaseComparison) -> LexicalRelation {
        if self.policy() != other.policy() {
            return LexicalRelation::Unresolved;
        }
        let Some(a) = self.anchor_key(model) else {
            return LexicalRelation::Unresolved;
        };
        let Some(b) = other.anchor_key(model) else {
            return LexicalRelation::Unresolved;
        };
        let Some(x) = self.relative.key(model) else {
            return LexicalRelation::Unresolved;
        };
        let Some(y) = other.relative.key(model) else {
            return LexicalRelation::Unresolved;
        };
        if a != b {
            return LexicalRelation::Separate;
        }
        if x == y {
            LexicalRelation::Equal
        } else if y.starts_with(&x) {
            LexicalRelation::Ancestor
        } else if x.starts_with(&y) {
            LexicalRelation::Descendant
        } else {
            LexicalRelation::Separate
        }
    }
    fn anchor_key(&self, model: CaseComparison) -> Option<AnchorKey> {
        if model == CaseComparison::Unknown {
            return None;
        }
        Some(match &self.anchor {
            TargetAnchor::PosixRoot => AnchorKey::Posix,
            TargetAnchor::WindowsDrive(letter) => AnchorKey::Drive(byte_key(&[*letter], model)?),
            TargetAnchor::WindowsUnc { server, share } => {
                AnchorKey::Unc(byte_key(server, model)?, byte_key(share, model)?)
            }
        })
    }
}
pub(super) fn byte_key(bytes: &[u8], model: CaseComparison) -> Option<Vec<u8>> {
    match model {
        CaseComparison::ByteSensitive => Some(bytes.to_vec()),
        CaseComparison::AsciiInsensitive if bytes.is_ascii() => {
            Some(bytes.iter().map(u8::to_ascii_lowercase).collect())
        }
        _ => None,
    }
}

#[cfg(test)]
mod macos_tests {
    use super::*;

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY",
        r = 2
    )]
    fn macos_posix_bytes_are_lexical_without_host_path_inference() {
        let policy = TargetPathPolicy::MacosPosixBytes;
        let path = AbsoluteTargetPath::new(
            policy,
            TargetAnchor::PosixRoot,
            vec![b"Library".to_vec(), vec![0xff, b':', b'\\', 0x80]],
        )
        .unwrap();
        assert_eq!(path.policy(), policy);
        assert_eq!(path.components()[1].bytes(), &[0xff, b':', b'\\', 0x80]);
        assert!(TargetComponent::new(policy, b"..".to_vec()).is_err());
        assert!(AbsoluteTargetPath::new(policy, TargetAnchor::WindowsDrive(b'C'), vec![]).is_err());
    }
}
