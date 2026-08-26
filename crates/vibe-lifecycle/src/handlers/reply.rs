use std::path::Path;

use serde_json::Deserializer;
use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle::e1::reply::Reply;

use super::{HandlerError, REPLY_CAP};

pub(super) fn parse_reply(bytes: &[u8], key: &str) -> Result<Reply, HandlerError> {
    if bytes.is_empty() || bytes.len() > REPLY_CAP {
        return Err(HandlerError::Reply {
            key: key.into(),
            reason: "empty or >1 MiB reply".into(),
            streams: None,
        });
    }
    let mut stream = Deserializer::from_slice(bytes).into_iter::<Reply>();
    let reply = stream
        .next()
        .ok_or_else(|| HandlerError::Reply {
            key: key.into(),
            reason: "empty reply".into(),
            streams: None,
        })?
        .map_err(|error| HandlerError::Reply {
            key: key.into(),
            reason: error.to_string(),
            streams: None,
        })?;
    if stream.next().is_some() {
        return Err(HandlerError::Reply {
            key: key.into(),
            reason: "stdout contains more than one JSON document".into(),
            streams: None,
        });
    }
    Ok(reply)
}

pub(crate) fn validate_reply(
    reply: &Reply,
    context: &Context,
    key: &str,
) -> Result<(), HandlerError> {
    if reply.envelope != 1 || !reply.tasks.is_empty() {
        return Err(HandlerError::Reply {
            key: key.into(),
            reason: "reply epoch/tasks invalid".into(),
            streams: None,
        });
    }
    // The generic row law is shared with agent preparation, which applies it
    // before a token is spent. One owner, so the pre-spend copy cannot drift.
    crate::artifacts::validate_shape(&reply.artifacts, &context.artifacts, &context.project.root)
        .map_err(|reason| HandlerError::Reply {
        key: key.into(),
        reason,
        streams: None,
    })?;
    if reply.artifacts.is_empty() {
        return Ok(());
    }
    let root = Path::new(&context.project.root)
        .canonicalize()
        .map_err(|error| HandlerError::Reply {
            key: key.into(),
            reason: error.to_string(),
            streams: None,
        })?;
    // What only the post-write caller can see: the physical file behind each
    // row. The lexical shape above is already proven.
    let mut paths = std::collections::BTreeSet::new();
    for artifact in &reply.artifacts {
        // Physical identity, not the raw spelling: two case-fold aliases of
        // one file are one artifact.
        let path =
            Path::new(&artifact.path)
                .canonicalize()
                .map_err(|error| HandlerError::Reply {
                    key: key.into(),
                    reason: error.to_string(),
                    streams: None,
                })?;
        if !path.starts_with(&root) {
            return Err(HandlerError::Reply {
                key: key.into(),
                reason: "artifact escapes selected project".into(),
                streams: None,
            });
        }
        if !paths.insert(path.clone())
            || context.artifacts.iter().any(|prior| {
                Path::new(&prior.path)
                    .canonicalize()
                    .is_ok_and(|prior| prior == path)
            })
        {
            return Err(HandlerError::Reply {
                key: key.into(),
                reason: format!("duplicate artifact path `{}`", artifact.path),
                streams: None,
            });
        }
        refuse_artifact_links(
            Path::new(&context.project.root),
            Path::new(&artifact.path),
            key,
        )?;
    }
    Ok(())
}

fn refuse_artifact_links(root: &Path, artifact: &Path, key: &str) -> Result<(), HandlerError> {
    let relative = artifact
        .strip_prefix(root)
        .map_err(|_| HandlerError::Reply {
            key: key.into(),
            reason: "artifact is not lexically below selected project".into(),
            streams: None,
        })?;
    if !relative
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(HandlerError::Reply {
            key: key.into(),
            reason: "artifact path contains non-Normal relative components".into(),
            streams: None,
        });
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|error| HandlerError::Reply {
                key: key.into(),
                reason: error.to_string(),
                streams: None,
            })?;
        if metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(HandlerError::Reply {
                key: key.into(),
                reason: format!("artifact traverses symlink/reparse `{}`", current.display()),
                streams: None,
            });
        }
    }
    Ok(())
}

fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}
