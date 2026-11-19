//! Durable reference protocol. A v2 empty roster is genesis evidence, not a
//! statement about the current binary's compiled consumer count.
use super::*;

pub(super) fn validate_format(index: &Index) -> Result<(), Error> {
    match (index.schema, index.reference_protocol, &index.participants) {
        (1, None, None) => Ok(()),
        (2, Some(1), Some(participants))
            if participants.len() <= 1024
                && participants.windows(2).all(|w| w[0] < w[1])
                && participants.iter().all(|s| safe_id(s)) =>
        {
            Ok(())
        }
        _ => Err(Error::Invalid),
    }
}
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:".contains(&b))
}

struct Inventory<'a>(&'a Store);
struct Guard {
    _inventory: vibe_safefs::LockGuard,
    participants: Vec<String>,
}
impl ReferenceInventory for Inventory<'_> {
    fn acquire(&self) -> Result<Box<dyn ReferenceGuard + '_>, Error> {
        let (project, root) = self.0.open(false)?.ok_or(Error::NotFound)?;
        let inventory = root
            .private_lock("inventory.lock")
            .map_err(|_| Error::RefusedUnknown)?;
        let lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::RefusedUnknown)?;
        let index = Store::load(&project, &root)?;
        if index.schema != 2 || index.reference_protocol != Some(1) {
            return Err(Error::RefusedUnknown);
        }
        let participants = index.participants.ok_or(Error::RefusedUnknown)?;
        // There are no admitted production consumer owners yet. A future owner
        // must be implemented here before its durable participant can be read.
        if !participants.is_empty() {
            return Err(Error::RefusedUnknown);
        }
        drop(lock);
        Ok(Box::new(Guard {
            _inventory: inventory,
            participants,
        }))
    }
}
impl ReferenceGuard for Guard {
    fn consumers(&self, _: &SecretId) -> Result<Vec<String>, Error> {
        if !self.participants.is_empty() {
            return Err(Error::RefusedUnknown);
        }
        Ok(Vec::new())
    }
}
impl Store {
    /// Trusted built-in composition. Only a witnessed schema2/protocol1 empty
    /// durable roster authorizes no references; v1/unknown owners refuse.
    pub fn remove_coordinated(
        &self,
        id: &SecretId,
        expected: &Expected,
    ) -> Result<MetadataSnapshot, Error> {
        self.remove(id, expected, Some(&Inventory(self)))
    }
}
