//! Pure validation of generated stream values against already owner-selected facts.
//! ExpectedBinding is binding data, never admission or an authority token. The
//! owner must independently prove live scope, safe stop and retained domain truth.
//! No reference is interpreted as a path, credential, grant or completion proof.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM");

use std::fmt;
use vibe_wire::generated::machine::e1::operation_stream_message::*;

pub const MAX_FRAME_BYTES: usize = 65_536;
pub const MAX_REPLAY_EVENTS: usize = 64;

/// Facts selected by the incumbent owner before validation, not by this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedBinding<'a> {
    pub context_id: &'a str,
    pub target_id: Option<&'a str>,
    pub operation_id: &'a str,
    pub run_id: &'a str,
    pub stream_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamValidationError {
    FrameLimit,
    InvalidShape,
    UnsupportedEpoch,
    ScopeMismatch,
    UnknownRun,
    InvalidValue(&'static str),
}
impl fmt::Display for StreamValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stream value refused: {self:?}")
    }
}
impl std::error::Error for StreamValidationError {}
type Check<T = ()> = Result<T, StreamValidationError>;
fn require(ok: bool, reason: &'static str) -> Check {
    if ok {
        Ok(())
    } else {
        Err(StreamValidationError::InvalidValue(reason))
    }
}
fn opaque(value: &str, cap: usize) -> Check {
    require(
        !value.is_empty()
            && value.len() <= cap
            && value.trim() == value
            && !value.chars().any(char::is_control),
        "invalid opaque value",
    )
}
fn reference(value: Option<&str>) -> Check {
    value.map_or(Ok(()), |v| opaque(v, 512))
}
fn decimal(value: &str, positive: bool) -> Check<u64> {
    require(
        !value.is_empty()
            && value.bytes().all(|b| b.is_ascii_digit())
            && (value.len() == 1 || !value.starts_with('0')),
        "noncanonical sequence",
    )?;
    let parsed = value
        .parse::<u64>()
        .map_err(|_| StreamValidationError::InvalidValue("u64 overflow"))?;
    require(!positive || parsed > 0, "zero event sequence")?;
    Ok(parsed)
}
// Counting writer stops serialization at the cap instead of allocating a
// second unbounded buffer for an already typed caller value.
fn size(value: &OperationStreamMessage) -> Check<usize> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let next = self
                .0
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_FRAME_BYTES)
                .ok_or_else(|| std::io::Error::other("stream frame cap"))?;
            self.0 = next;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| StreamValidationError::FrameLimit)?;
    Ok(counter.0)
}

/// Check input byte size before generated decoding, including whitespace bytes.
/// Successful parsing and validation still do not admit any operation or control.
pub fn parse(bytes: &[u8], expected: ExpectedBinding<'_>) -> Check<OperationStreamMessage> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(StreamValidationError::FrameLimit);
    }
    let message = serde_json::from_slice(bytes).map_err(|_| StreamValidationError::InvalidShape)?;
    validate(&message, expected)?;
    Ok(message)
}

/// Validate shape-derived typed data; callers still own admission and evidence.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM",
    r = 1
)]
pub fn validate(message: &OperationStreamMessage, expected: ExpectedBinding<'_>) -> Check {
    if size(message)? > MAX_FRAME_BYTES {
        return Err(StreamValidationError::FrameLimit);
    }
    if message.schema != 1 {
        return Err(StreamValidationError::UnsupportedEpoch);
    }
    for id in [
        &message.context_id,
        &message.operation_id,
        &message.run_id,
        &message.stream_id,
    ] {
        opaque(id, 256)?;
    }
    if let Some(id) = &message.target_id {
        opaque(id, 256)?;
    }
    require(
        (message.operation_id.contains(':') || message.operation_id.contains('.'))
            && !message.operation_id.chars().any(char::is_whitespace),
        "operation must be namespaced",
    )?;
    if message.run_id != expected.run_id {
        return Err(StreamValidationError::UnknownRun);
    }
    if message.context_id != expected.context_id
        || message.target_id.as_deref() != expected.target_id
        || message.operation_id != expected.operation_id
        || message.stream_id != expected.stream_id
    {
        return Err(StreamValidationError::ScopeMismatch);
    }
    match &message.body {
        Body::Event(row) => {
            event(&row.event)?;
        }
        Body::Gap(row) => {
            gap(
                &row.from_sequence,
                &row.to_sequence,
                row.terminal_ref.as_deref(),
            )?;
        }
        Body::ReplayRequest(row) => {
            decimal(&row.after_sequence, false)?;
            require(
                (1..=64).contains(&row.max_events) && (1..=65_536).contains(&row.max_bytes),
                "replay request limits",
            )?;
        }
        Body::ReplayReply(row) => replay(row)?,
        Body::ControlRequest(row) => {
            opaque(&row.request_id, 256)?;
            decimal(&row.after_sequence, false)?;
        }
        Body::ControlReply(row) => control(row)?,
    }
    Ok(())
}
fn event(row: &EventRecord) -> Check<u64> {
    let sequence = decimal(&row.sequence, true)?;
    match &row.payload {
        Payload::State(p) => {
            reference(p.result_ref.as_deref())?;
            require(
                !matches!(
                    p.observed,
                    PayloadStateObserved::JournalReady | PayloadStateObserved::Running
                ) || p.effect_state != PayloadStateEffectState::Completed,
                "active state claims completed operation",
            )?;
            require(
                (p.effect_state != PayloadStateEffectState::Completed
                    && p.observed != PayloadStateObserved::ResultAvailable)
                    || p.result_ref.is_some(),
                "state lacks result reference",
            )?;
        }
        Payload::Phase(p) => {
            opaque(&p.phase, 256)?;
            match p.transition {
                PayloadPhaseTransition::Started => {
                    require(p.status.is_none(), "started phase has status")?
                }
                PayloadPhaseTransition::Finished => opaque(
                    p.status
                        .as_deref()
                        .ok_or(StreamValidationError::InvalidValue(
                            "finished phase lacks status",
                        ))?,
                    256,
                )?,
            }
        }
        Payload::Progress(p) => {
            let current = decimal(&p.current, false)?;
            if let Some(total) = &p.total {
                require(current <= decimal(total, false)?, "progress exceeds total")?;
            }
        }
        Payload::Diagnostic(p) => {
            require(p.message.len() <= 4096, "diagnostic byte cap")?;
        }
        Payload::RawOutput(p) => {
            let offset = decimal(&p.byte_offset, false)?;
            require(
                offset.checked_add(u64::from(p.byte_count)).is_some(),
                "raw byte end overflow",
            )?;
            require(
                p.hex_bytes.len() <= 4096
                    && p.hex_bytes.len() % 2 == 0
                    && p.hex_bytes
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    && p.byte_count <= 2048
                    && p.byte_count as usize == p.hex_bytes.len() / 2,
                "raw byte encoding/count",
            )?;
        }
        Payload::TerminalReference(p) => opaque(&p.result_ref, 512)?,
    }
    Ok(sequence)
}
fn gap(from: &str, to: &str, terminal_ref: Option<&str>) -> Check<(u64, u64)> {
    let from = decimal(from, true)?;
    let to = decimal(to, true)?;
    require(from <= to, "reversed gap")?;
    reference(terminal_ref)?;
    Ok((from, to))
}
fn replay(row: &BodyReplayReply) -> Check {
    let after = decimal(&row.after_sequence, false)?;
    let newest = row
        .newest_sequence
        .as_deref()
        .map(|v| decimal(v, false))
        .transpose()?;
    require(newest.is_none_or(|n| n >= after), "newest precedes cursor")?;
    require(
        row.expired || newest.is_some(),
        "live reply lacks known tip",
    )?;
    require(
        row.events.len() <= MAX_REPLAY_EVENTS
            && serde_json::to_vec(&row.events)
                .map_err(|_| StreamValidationError::InvalidShape)?
                .len()
                <= MAX_FRAME_BYTES,
        "replay batch cap",
    )?;
    reference(row.terminal_ref.as_deref())?;
    if row.expired {
        require(row.events.is_empty(), "expired history contains payload")?;
    }
    require(
        row.expired || !row.events.is_empty() || row.gap.is_some() || newest == Some(after),
        "known history omitted without payload or gap",
    )?;
    let mut previous = after;
    if let Some(g) = &row.gap {
        let (from, to) = gap(&g.from_sequence, &g.to_sequence, g.terminal_ref.as_deref())?;
        require(
            after.checked_add(1) == Some(from),
            "gap does not begin after cursor",
        )?;
        require(newest.is_none_or(|n| to <= n), "gap exceeds tip")?;
        require(
            !row.expired || g.reason == GapReason::Expired,
            "expiry gap reason",
        )?;
        require(
            row.expired || g.reason != GapReason::Expired,
            "live reply claims expired gap",
        )?;
        previous = to;
    }
    for e in &row.events {
        let seq = event(e)?;
        require(
            previous.checked_add(1) == Some(seq),
            "noncontiguous or overlapping replay",
        )?;
        require(newest.is_some_and(|n| seq <= n), "event exceeds known tip")?;
        previous = seq;
    }
    Ok(())
}
fn control(row: &BodyControlReply) -> Check {
    use BodyControlReplyAction as Action;
    use BodyControlReplyOperationEffectState as Effect;
    use BodyControlReplyStatus as Status;
    opaque(&row.request_id, 256)?;
    reference(row.result_ref.as_deref())?;
    match row.action {
        Action::Detach => require(
            matches!(
                row.status,
                Status::Requested | Status::Unavailable | Status::Detached
            ),
            "detach claims provider stop",
        )?,
        Action::Cancel => require(
            row.status != Status::Detached,
            "cancel claims observer detach",
        )?,
    }
    if row.status == Status::AlreadyTerminal {
        require(row.result_ref.is_some(), "terminal reply lacks reference")?;
    }
    if row.operation_effect_state == Effect::Completed {
        require(
            matches!(row.status, Status::AlreadyTerminal | Status::Detached)
                && row.result_ref.is_some(),
            "control acknowledgment claims completion",
        )?;
    }
    if row.status == Status::ReachedSafeStop {
        require(
            row.operation_effect_state == Effect::NoEffects
                || (row.operation_effect_state == Effect::Uncertain && row.result_ref.is_some()),
            "safe stop contradicts retained effects",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
