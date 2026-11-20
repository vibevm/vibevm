//! Closed value-free decoding before request-local Doc definition admission.
use crate::generated::read_doc_request::ReadDocRequest;
use serde::de::Error as _;
pub fn parse(value: serde_json::Value) -> Result<ReadDocRequest, serde_json::Error> {
    let invalid = || {
        serde_json::Error::custom(
            "read_doc request: invalid shape; expected address string, optional format md/xml, optional lang string and non-null input.selections exact single-member records",
        )
    };
    if ["input", "format", "lang"]
        .iter()
        .any(|key| value.get(key).is_some_and(serde_json::Value::is_null))
    {
        return Err(invalid());
    }
    serde_json::from_value(value).map_err(|_| invalid())
}
