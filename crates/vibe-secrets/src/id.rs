//! Random logical identity, unrelated to the stored value or human label.
use crate::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SecretId(String);
impl SecretId {
    pub(crate) fn generate() -> Result<Self, Error> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| Error::Unavailable)?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self(format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SecretId {
    type Error = Error;
    fn try_from(raw: String) -> Result<Self, Error> {
        let valid = raw.len() == 36
            && raw.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                }
            })
            && raw.as_bytes()[14] == b'4'
            && b"89ab".contains(&raw.as_bytes()[19]);
        if !valid {
            return Err(Error::Invalid);
        }
        Ok(Self(raw))
    }
}
impl From<SecretId> for String {
    fn from(id: SecretId) -> Self {
        id.0
    }
}
