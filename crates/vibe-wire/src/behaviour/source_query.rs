//! Owning semantic validation beside the generated source-query shapes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-OBSERVATION-BINDING");

use crate::generated::source_query::e1::{
    error::Error,
    request::Request,
    result::{Result as QueryResult, ResultMode},
};
use serde_json::{Value, json};

pub const EPOCH: u32 = 1;
pub const MAX_CHUNK_BYTES: u32 = 65_536;
pub const MAX_CONTENT_BYTES: u32 = 64 * 1024 * 1024;
pub const MAX_CANDIDATES: usize = 128;
pub const MAX_CURSOR_BYTES: usize = 256;

fn cursor_in_bounds(cursor: &str) -> bool {
    !cursor.is_empty() && cursor.len() <= MAX_CURSOR_BYTES
}

fn outgoing_cursor_offset(cursor: &str) -> Option<u32> {
    if !cursor_in_bounds(cursor) {
        return None;
    }
    let mut parts = cursor.split('.');
    if parts.next()? != "q1" || !digest(parts.next()?, "") {
        return None;
    }
    let encoded = parts.next()?;
    let offset = encoded.parse::<u32>().ok()?;
    if parts.next().is_some() || offset == 0 || offset.to_string() != encoded {
        return None;
    }
    Some(offset)
}

pub fn digest(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|tail| {
        tail.len() == 64
            && tail
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

pub fn validate_request(request: &Request) -> std::result::Result<(), &'static str> {
    if request.schema != EPOCH {
        return Err("epoch");
    }
    if request.address.is_empty() || request.address.len() > 4096 {
        return Err("address");
    }
    if request
        .chunk_bytes
        .is_some_and(|size| size == 0 || size > MAX_CHUNK_BYTES)
    {
        return Err("chunk size");
    }
    if request
        .cursor
        .as_ref()
        .is_some_and(|cursor| !cursor_in_bounds(cursor))
    {
        return Err("cursor");
    }
    if request
        .expected_resolution_hash
        .as_deref()
        .is_some_and(|value| !digest(value, "sha256-query-resolution/1:"))
    {
        return Err("resolution expectation");
    }
    if request
        .expected_document_hash
        .as_deref()
        .is_some_and(|value| !digest(value, "sha256:"))
    {
        return Err("document expectation");
    }
    if request
        .expected_source_hash
        .as_deref()
        .is_some_and(|value| vibe_core::ContentHash::parse(value).is_err())
    {
        return Err("source expectation");
    }
    Ok(())
}

pub fn read_request(value: &Value) -> std::result::Result<Request, &'static str> {
    let request = serde_json::from_value(value.clone()).map_err(|_| "request shape")?;
    validate_request(&request)?;
    Ok(request)
}

pub fn validate_error(error: &Error) -> std::result::Result<(), &'static str> {
    if error.schema != EPOCH
        || error.message.is_empty()
        || error.message.len() > 4096
        || error.boundary.is_empty()
        || error.boundary.len() > 128
    {
        return Err("refusal boundary");
    }
    if error.candidates.len() > MAX_CANDIDATES
        || error
            .candidates
            .iter()
            .any(|value| value.is_empty() || value.len() > 4096)
    {
        return Err("refusal candidates");
    }
    Ok(())
}

pub fn read_error(value: &Value) -> std::result::Result<Error, &'static str> {
    let error = serde_json::from_value(value.clone()).map_err(|_| "refusal shape")?;
    validate_error(&error)?;
    Ok(error)
}

pub fn validate_result(result: &QueryResult) -> std::result::Result<(), &'static str> {
    if result.schema != EPOCH || result.address.is_empty() || result.address.len() > 4096 {
        return Err("result identity");
    }
    if result.location.line_start == 0 || result.location.line_end < result.location.line_start {
        return Err("native range");
    }
    if result.location.path.is_empty()
        || result.location.path.starts_with('/')
        || result.location.path.contains('\\')
        || result
            .location
            .path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err("native path");
    }
    if let Some(encoded) = &result.derivative_result_json {
        let record: crate::generated::derivative_result::DerivativeResult =
            serde_json::from_str(encoded).map_err(|_| "derivative record shape")?;
        record
            .checked_binding()
            .map_err(|_| "derivative record identity")?;
    }
    let observation = &result.observation;
    if !digest(&observation.resolution_hash, "sha256-query-resolution/1:")
        || !digest(&observation.document_hash, "sha256:")
        || !digest(&observation.manifest_hash, "sha256:")
        || observation
            .semantic_hash
            .as_deref()
            .is_some_and(|value| !digest(value, "sha256-specdoc-node/1:"))
        || observation.descriptor_identity.is_empty()
        || observation.representation_recipe.is_empty()
    {
        return Err("observation identity");
    }
    if observation
        .source_hash
        .as_deref()
        .is_some_and(|value| vibe_core::ContentHash::parse(value).is_err())
    {
        return Err("source identity");
    }
    let chunk = &result.chunk;
    let total = if result.mode == ResultMode::Read {
        result.selected_bytes
    } else {
        0
    };
    if chunk.offset > chunk.end
        || chunk.end > chunk.total
        || chunk.total != total
        || chunk.end - chunk.offset != result.content.len() as u32
        || result.content.len() > MAX_CHUNK_BYTES as usize
        || result.selected_bytes > MAX_CONTENT_BYTES
        || result.document_bytes > MAX_CONTENT_BYTES
        || observation.captured_bytes > MAX_CONTENT_BYTES
    {
        return Err("chunk bounds");
    }
    if (chunk.end < chunk.total) != chunk.next_cursor.is_some()
        || chunk
            .next_cursor
            .as_deref()
            .is_some_and(|cursor| outgoing_cursor_offset(cursor) != Some(chunk.end))
    {
        return Err("chunk continuation");
    }
    if result.heading_chain.len() > 64
        || result.outline.len() > 16_384
        || result.outline.iter().any(|node| {
            node.path.len() > 64
                || node.heading_chain.len() > 64
                || node.bytes_without_children > node.bytes_with_children
                || node.size_recipe != "canonical-markdown+specdoc-selection/1"
        })
    {
        return Err("outline bounds");
    }
    Ok(())
}

pub fn read_result(value: &Value) -> std::result::Result<QueryResult, &'static str> {
    let result = serde_json::from_value(value.clone()).map_err(|_| "result shape")?;
    validate_result(&result)?;
    Ok(result)
}

/// The one descriptor projection for this fixed primitive-field request.
/// Shape is read from the registered JTD; no second handwritten field list.
pub fn request_json_schema() -> Value {
    primitive_request_schema(include_str!(
        "../../../../schemas/source_query/e1/request.jtd.json"
    ))
}

pub fn doc_request_json_schema() -> Value {
    primitive_request_schema(include_str!(
        "../../../../schemas/source_query/e1/doc_request.jtd.json"
    ))
}

fn primitive_request_schema(source: &str) -> Value {
    let jtd: Value = serde_json::from_str(source).expect("registered source request JTD");
    let mut properties = serde_json::Map::new();
    for group in ["properties", "optionalProperties"] {
        for (name, definition) in jtd[group].as_object().expect("request properties") {
            let property = if let Some(values) = definition.get("enum") {
                json!({"type":"string","enum":values})
            } else {
                match definition["type"]
                    .as_str()
                    .expect("primitive request field")
                {
                    "string" => json!({"type":"string"}),
                    "boolean" => json!({"type":"boolean"}),
                    "uint32" => json!({"type":"integer","minimum":0,"maximum":u32::MAX}),
                    _ => unreachable!("source request uses fixed primitive shapes"),
                }
            };
            properties.insert(name.clone(), property);
        }
    }
    json!({"type":"object","properties":properties,"required":jtd["properties"].as_object().unwrap().keys().collect::<Vec<_>>(),"additionalProperties":false})
}

pub fn read_doc_request(
    value: &Value,
) -> std::result::Result<crate::generated::source_query::e1::doc_request::DocRequest, &'static str>
{
    let request: crate::generated::source_query::e1::doc_request::DocRequest =
        serde_json::from_value(value.clone()).map_err(|_| "documentation request shape")?;
    if request.schema != EPOCH {
        return Err("epoch");
    }
    if request.address.is_empty() || request.address.len() > 4096 {
        return Err("documentation address");
    }
    if request
        .lang
        .as_ref()
        .is_some_and(|lang| lang.is_empty() || lang.len() > 128)
    {
        return Err("documentation language");
    }
    Ok(request)
}

pub fn read_doc_result(
    value: &Value,
) -> std::result::Result<crate::generated::source_query::e1::doc_result::DocResult, &'static str> {
    let result: crate::generated::source_query::e1::doc_result::DocResult =
        serde_json::from_value(value.clone()).map_err(|_| "documentation result shape")?;
    if result.schema != EPOCH
        || result.page.len() != result.page_bytes as usize
        || result.page_bytes > MAX_CONTENT_BYTES
        || result.lang.is_empty()
        || result.lang.len() > 128
    {
        return Err("documentation projection bounds");
    }
    let selection = read_result(
        &serde_json::to_value(&result.selection).map_err(|_| "selected source encoding")?,
    )?;
    if selection.mode != ResultMode::Resolve || result.address != selection.address {
        return Err("documentation selected source");
    }
    Ok(result)
}

#[cfg(test)]
mod m_01_d_2_cases {
    use super::*;
    #[test]
    fn registered_corpora_distinguish_shapes_and_semantic_boundaries() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora");
        let mut counts = [0; 3];
        for name in ["request", "result", "error"] {
            for (index, class) in ["valid", "structural-invalid", "semantic-gap"]
                .iter()
                .enumerate()
            {
                let directory = root.join(format!("source-query-{name}/e1/{class}"));
                for file in std::fs::read_dir(directory).unwrap() {
                    let file = file.unwrap().path();
                    let value: Value =
                        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
                    let (shape, accepted) = match name {
                        "request" => (
                            serde_json::from_value::<Request>(value.clone()).is_ok(),
                            read_request(&value).is_ok(),
                        ),
                        "result" => (
                            serde_json::from_value::<QueryResult>(value.clone()).is_ok(),
                            read_result(&value).is_ok(),
                        ),
                        "error" => (
                            serde_json::from_value::<Error>(value.clone()).is_ok(),
                            read_error(&value).is_ok(),
                        ),
                        _ => unreachable!(),
                    };
                    assert_eq!(accepted, index == 0, "{}", file.display());
                    assert_eq!(shape, index != 1, "{}", file.display());
                    counts[index] += 1;
                }
            }
        }
        assert_eq!(counts, [3, 3, 6]);
    }
    #[test]
    fn registered_doc_contract_preserves_closed_epoch_and_projection_roles() {
        use crate::generated::source_query::e1::{doc_request::DocRequest, doc_result::DocResult};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora");
        let mut counts = [0; 3];
        for name in ["request", "result"] {
            for (index, class) in ["valid", "structural-invalid", "semantic-gap"]
                .iter()
                .enumerate()
            {
                for file in
                    std::fs::read_dir(root.join(format!("source-doc-{name}/e1/{class}"))).unwrap()
                {
                    let file = file.unwrap().path();
                    let value: Value =
                        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
                    let (shape, accepted) = if name == "request" {
                        (
                            serde_json::from_value::<DocRequest>(value.clone()).is_ok(),
                            read_doc_request(&value).is_ok(),
                        )
                    } else {
                        (
                            serde_json::from_value::<DocResult>(value.clone()).is_ok(),
                            read_doc_result(&value).is_ok(),
                        )
                    };
                    assert_eq!(shape, index != 1, "{}", file.display());
                    assert_eq!(accepted, index == 0, "{}", file.display());
                    counts[index] += 1;
                }
            }
        }
        assert_eq!(counts, [2, 2, 4]);
    }

    #[test]
    fn outgoing_continuation_refuses_empty_oversized_and_malformed_cursor() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/source-query-result/e1/valid/representative.json");
        let mut value: Value = serde_json::from_slice(&std::fs::read(root).unwrap()).unwrap();
        value["content"] = json!("## Solu");
        value["chunk"]["end"] = json!(7);
        let good = format!("q1.{}.7", "a".repeat(64));
        value["chunk"]["next_cursor"] = json!(good);
        assert!(read_result(&value).is_ok());
        for cursor in [
            "".to_owned(),
            "x".repeat(257),
            "not-q1".to_owned(),
            format!("q1.{}.07", "a".repeat(64)),
            format!("q1.{}.8", "a".repeat(64)),
        ] {
            value["chunk"]["next_cursor"] = json!(cursor);
            assert_eq!(
                read_result(&value).err(),
                Some("chunk continuation"),
                "{cursor:?}"
            );
        }
        let mut request = json!({"schema":1,"address":"spec://org.demo/self/DOC","mode":"read","format":"md","children":true});
        for cursor in ["".to_owned(), "x".repeat(257)] {
            request["cursor"] = json!(cursor);
            assert_eq!(read_request(&request).err(), Some("cursor"));
        }
    }

    #[test]
    fn generated_request_descriptor_and_refusal_are_closed_and_epoch_bound() {
        let valid = json!({"schema":1,"address":"spec://org.demo/self/DOC#solution","mode":"read","format":"md","children":true});
        let request = read_request(&valid).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), valid);
        let descriptor = request_json_schema();
        assert_eq!(descriptor["additionalProperties"], false);
        for key in valid.as_object().unwrap().keys() {
            assert!(descriptor["properties"].get(key).is_some());
        }
        let mut foreign = valid.clone();
        foreign["root"] = json!("/foreign");
        assert_eq!(read_request(&foreign).unwrap_err(), "request shape");
        let mut epoch = valid;
        epoch["schema"] = json!(2);
        assert_eq!(read_request(&epoch).unwrap_err(), "epoch");
        let refusal = json!({"schema":1,"code":"unknown_anchor","message":"Unknown exact node","boundary":"node selection","candidates":["root.solution"]});
        assert!(read_error(&refusal).is_ok());
        let mut foreign = refusal;
        foreign["effects"] = json!("none");
        assert!(read_error(&foreign).is_err());
    }
}
