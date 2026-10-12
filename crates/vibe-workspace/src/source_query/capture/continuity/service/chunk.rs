//! Exact UTF-8 chunks tied to one request and observation.
use super::*;

pub(super) fn read(
    content: &str,
    request: &Request,
    resolution: &str,
    document: &str,
    observation_identity: &str,
) -> Result<(String, Value), QueryError> {
    let mut shape =
        serde_json::to_value(request).map_err(|_| unavailable("cursor request encoding"))?;
    shape.as_object_mut().unwrap().remove("cursor");
    let size = request.chunk_bytes.unwrap_or(wire::MAX_CHUNK_BYTES) as usize;
    let encoded = serde_json::to_vec(&(shape, resolution, document, observation_identity, content))
        .map_err(|_| unavailable("cursor observation encoding"))?;
    let identity = format!("{:x}", Sha256::digest(encoded));
    let offset = match request.cursor.as_deref() {
        None => 0,
        Some(cursor) => {
            let parts = cursor.split('.').collect::<Vec<_>>();
            if parts.len() != 3 || parts[0] != "q1" || parts[1] != identity {
                return Err(refusal(
                    ErrorCode::StaleCursor,
                    "chunk cursor",
                    "Cursor does not bind this request/observation",
                    vec![],
                ));
            }
            let offset = parts[2].parse::<usize>().map_err(|_| {
                refusal(
                    ErrorCode::StaleCursor,
                    "chunk cursor",
                    "Invalid cursor offset",
                    vec![],
                )
            })?;
            if offset.to_string() != parts[2]
                || offset >= content.len()
                || !content.is_char_boundary(offset)
            {
                return Err(refusal(
                    ErrorCode::StaleCursor,
                    "chunk cursor",
                    "Cursor offset is outside this representation",
                    vec![],
                ));
            }
            offset
        }
    };
    let mut end = (offset + size).min(content.len());
    while end > offset && !content.is_char_boundary(end) {
        end -= 1;
    }
    if end == offset && end < content.len() {
        return Err(refusal(
            ErrorCode::LimitExceeded,
            "chunk UTF-8 boundary",
            "Chunk budget cannot hold the next UTF-8 scalar",
            vec![],
        ));
    }
    let next = (end < content.len()).then(|| format!("q1.{identity}.{end}"));
    Ok((
        content[offset..end].to_owned(),
        json!({"offset":offset,"end":end,"total":content.len(),"next_cursor":next}),
    ))
}
