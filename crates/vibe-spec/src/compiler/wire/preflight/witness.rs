//! Gate 11 as a whole-carrier phase: SET PROJECTION. The witness gates that
//! follow it (12, 13, 14) are replays over the constructed value and live in
//! `wire::staged`; the closure decoder used to run 14 before 12/13 and 11
//! last of all, which is the schema's set of gates in the wrong order.

use crate::compiler::wire::{G_SET_PROJECTION, IrWireError, gate, wire};

use super::closure;

/// Gate 11: every wire list projecting a domain `BTreeSet<String>` carries
/// its canonical sorted, duplicate-free spelling.
pub(super) fn sets(ir: &wire::Ir) -> Result<(), IrWireError> {
    let Some(value) = closure(ir) else {
        return Ok(());
    };
    if let Some(snapshot) = &value.pending_sources {
        check_set("pending_sources", &snapshot.explicit_use_keys)?;
    }
    if let Some(snapshot) = &value.pending_embeds {
        check_set("pending_embeds", &snapshot.explicit_use_keys)?;
    }
    Ok(())
}

pub(in crate::compiler::wire) fn check_set(
    site: &'static str,
    values: &[String],
) -> Result<(), IrWireError> {
    let sorted = values.windows(2).all(|pair| pair[0] < pair[1]);
    if sorted {
        Ok(())
    } else {
        Err(gate(
            G_SET_PROJECTION,
            format!("{site} explicit_use_keys must be sorted and duplicate-free"),
        ))
    }
}
