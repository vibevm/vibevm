//! Structured query routes for `/v1/packages*`.
//!
//! A package is addressed by its `(group, name)` identity (PROP-008
//! §2.2): the path shape is `/v1/packages/{group}/{name}[/{version}]`.
//! `kind` is metadata and never keys a route.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#index-ext");

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use chrono::Utc;
use semver::Version;
use serde::{Deserialize, Serialize};
use vibe_core::Group;

use crate::error::Error;
use crate::index::Index;
use crate::index::memory::{WriteCtx, default_generator};
use crate::index::search;
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};
use crate::server::error::ApiError;
use crate::server::state::AppState;
use crate::types::{PackageKind, VersionEntry};

#[derive(Debug, Deserialize, Default)]
pub struct ListSearchQuery {
    pub kind: Option<PackageKind>,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub command: &'static str,
    pub registry: String,
    pub package_count: u32,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub packages: Vec<PackageRow>,
}

#[derive(Debug, Serialize)]
pub struct PackageRow {
    /// `kind` is metadata (PROP-008 §2.3) — read from the package's
    /// versions; `None` only for a zero-version package row.
    pub kind: Option<PackageKind>,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub versions: Vec<Version>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub command: &'static str,
    pub query: String,
    pub hit_count: usize,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub score: u32,
    pub matched_tokens: Vec<String>,
    pub description: Option<String>,
}

/// Parse a `{group}` path segment into a validated [`Group`]. A
/// malformed segment is a 400 — the URL itself is syntactically wrong.
fn parse_group(s: &str) -> Result<Group, ApiError> {
    Group::parse(s).map_err(|e| {
        ApiError::bad_request(format!(
            "invalid group `{s}`: {e} (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
             fix: use a reverse-FQDN group like `org.vibevm`)"
        ))
    })
}

/// The package's `kind` — metadata carried per version (PROP-008 §2.3).
/// `None` only for the rare zero-version package row.
fn package_kind(pkg: &crate::types::PackageEntry) -> Option<PackageKind> {
    pkg.versions.first().map(|v| v.kind)
}

pub async fn list_or_search(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListSearchQuery>,
) -> Result<axum::response::Response, ApiError> {
    state.stats.note_request();
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);
    let index = state.index.read().await;

    if let Some(query) = &q.q {
        let hits = search::search(&index, query, q.kind);
        let hits: Vec<SearchHit> = hits
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|h| SearchHit {
                kind: h.kind,
                group: h.group,
                name: h.name,
                latest_stable: h.latest_stable,
                score: h.score,
                matched_tokens: h.matched_tokens,
                description: h.description,
            })
            .collect();
        let body = SearchResponse {
            command: "search",
            query: query.clone(),
            hit_count: hits.len(),
            hits,
        };
        return Ok(Json(body).into_response());
    }

    // List mode. `kind` is per-version metadata, so the `?kind=` filter
    // keeps a package iff any of its versions carries that kind.
    let mut rows: Vec<PackageRow> = index
        .by_pkgref
        .values()
        .filter(|p| {
            q.kind
                .is_none_or(|k| p.versions.iter().any(|v| v.kind == k))
        })
        .map(|p| PackageRow {
            kind: package_kind(p),
            group: p.group.clone(),
            name: p.name.clone(),
            latest_stable: p.latest_stable.clone(),
            versions: p.versions.iter().map(|v| v.version.clone()).collect(),
            description: p.versions.last().and_then(|v| v.description.clone()),
        })
        .collect();
    rows.sort_by(|a, b| a.group.cmp(&b.group).then(a.name.cmp(&b.name)));
    let package_count = rows.len() as u32;
    let returned: Vec<PackageRow> = rows.into_iter().skip(offset).take(limit).collect();
    let body = ListResponse {
        command: "list",
        registry: index.registry.clone(),
        package_count,
        returned: returned.len(),
        offset,
        limit,
        packages: returned,
    };
    Ok(Json(body).into_response())
}

pub async fn package_versions(
    State(state): State<Arc<AppState>>,
    Path((group_str, name)): Path<(String, String)>,
) -> Result<Json<PackageVersionsResponse>, ApiError> {
    state.stats.note_request();
    let group = parse_group(&group_str)?;
    let index = state.index.read().await;
    let pkg = index
        .get(&group, &name)
        .ok_or_else(|| ApiError::not_found(format!("`{group}/{name}` is not in the index (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: check the (group, name) identity, or publish the package first)")))?;
    Ok(Json(PackageVersionsResponse {
        command: "package",
        kind: package_kind(pkg),
        group: pkg.group.clone(),
        name: pkg.name.clone(),
        latest_stable: pkg.latest_stable.clone(),
        versions: pkg.versions.clone(),
    }))
}

#[derive(Serialize)]
pub struct PackageVersionsResponse {
    pub command: &'static str,
    pub kind: Option<PackageKind>,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub versions: Vec<VersionEntry>,
}

pub async fn single_version(
    State(state): State<Arc<AppState>>,
    Path((group_str, name, version_str)): Path<(String, String, String)>,
) -> Result<Json<VersionEntry>, ApiError> {
    state.stats.note_request();
    let group = parse_group(&group_str)?;
    let v: Version = version_str
        .parse()
        .map_err(|e| ApiError::bad_request(format!("`{version_str}` is not valid semver: {e} (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: request a semver version like `0.1.0`)")))?;
    let index = state.index.read().await;
    let pkg = index
        .get(&group, &name)
        .ok_or_else(|| ApiError::not_found(format!("`{group}/{name}` is not in the index (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: check the (group, name) identity, or publish the package first)")))?;
    let entry = pkg
        .versions
        .iter()
        .find(|e| e.version == v)
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "`{group}/{name}@{version_str}` is not in the index (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: GET the package to list its versions, then request one that exists)"
            ))
        })?
        .clone();
    Ok(Json(entry))
}

use axum::response::IntoResponse;

// ---------------------------------------------------------------------------
// Mutating endpoints (slice 6)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct UpsertResponse {
    pub command: &'static str,
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub created: bool,
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(entry): Json<VersionEntry>,
) -> Result<(StatusCode, Json<UpsertResponse>), ApiError> {
    require_writeable(&state, &headers)?;
    // Ф3.2c2 — the scope check's registry value is the projection's.
    // The in-memory copy IS a fold of the journal (boot and every
    // mutation replace it wholesale), so this reads the journal's
    // identity, never a catalog's.
    let registry = state.index.read().await.registry.clone();
    if entry.registry != registry {
        return Err(ApiError::bad_request(format!(
            "scope violation: entry.registry=`{}` differs from server registry=`{}` \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
             fix: POST only entries whose `registry` matches this server's)",
            entry.registry, registry
        )));
    }
    let kind = entry.kind;
    let group = entry.group.clone();
    let name = entry.name.clone();
    let version = entry.version.clone();

    // Ф3.2c2 — the fact is built up front so ONE value both answers
    // "did the state change" (`Index::upsert`, F2-3, whole-value
    // equality) and rides the journal. The probe fold it is applied to
    // is discarded: the persisted catalog comes from re-folding the
    // journal, never from the probe.
    let event = Event::Published {
        entry: Box::new(entry.clone()),
    };
    let (existed, changed) = mutate(&state, |probe| {
        let existed = probe
            .get(&group, &name)
            .map(|p| p.versions.iter().any(|v| v.version == version))
            .unwrap_or(false);
        let changed = probe.upsert(entry);
        (existed, changed.then_some(event))
    })
    .await?;

    // F2-3 — a mutation that changes nothing publishes nothing: the
    // response is still success, because the resource is already in
    // the requested state — which is what idempotency means over HTTP.
    if changed {
        state.stats.note_mutation();
        publish_mutation(&state, format!("index: upsert {group}/{name}@{version}")).await;
    }
    let status = if !existed {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(UpsertResponse {
            command: "upsert",
            kind,
            group,
            name,
            version,
            created: !existed,
        }),
    ))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub command: &'static str,
    pub group: Group,
    pub name: String,
    pub version: Option<Version>,
    pub removed: bool,
}

pub async fn delete_version(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_str, name, version_str)): Path<(String, String, String)>,
) -> Result<Json<DeleteResponse>, ApiError> {
    require_writeable(&state, &headers)?;
    let group = parse_group(&group_str)?;
    let v: Version = version_str
        .parse()
        .map_err(|e| ApiError::bad_request(format!("`{version_str}` is not valid semver: {e} (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: request a semver version like `0.1.0`)")))?;
    // Ф3.2c2 — "is there something to remove" is answered by folding
    // the journal BEFORE any record is appended: a removal of what
    // never stood in the projection would be a fact that never held,
    // and the journal carries no false facts.
    let (removed, changed) = mutate(&state, |probe| {
        let removed = probe.remove_version(&group, &name, &v);
        (
            removed,
            removed.then_some(Event::Removed {
                group: group.clone(),
                name: name.clone(),
                version: Some(v.clone()),
            }),
        )
    })
    .await?;
    if changed {
        state.stats.note_mutation();
        publish_mutation(&state, format!("index: remove {group}/{name}@{v}")).await;
    }
    Ok(Json(DeleteResponse {
        command: "delete",
        group,
        name,
        version: Some(v),
        removed,
    }))
}

pub async fn delete_package(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_str, name)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, ApiError> {
    require_writeable(&state, &headers)?;
    let group = parse_group(&group_str)?;
    let (removed, changed) = mutate(&state, |probe| {
        let removed = probe.remove_package(&group, &name);
        (
            removed,
            removed.then_some(Event::Removed {
                group: group.clone(),
                name: name.clone(),
                version: None,
            }),
        )
    })
    .await?;
    if changed {
        state.stats.note_mutation();
        publish_mutation(&state, format!("index: remove {group}/{name}")).await;
    }
    Ok(Json(DeleteResponse {
        command: "delete",
        group,
        name,
        version: None,
        removed,
    }))
}

/// One journal-first mutation, shared by all three write routes
/// (Ф3.2c2). The form is the CLI's — `validate → append → project →
/// write_to`, truth first (PROP-044 `##LAW-NO-UNRECOVERABLE`): the
/// record lands in the journal before the derived catalog is written,
/// so a failed `write_to` leaves a journal the next mutation re-folds,
/// never a catalog whose truth never existed.
///
/// The mutation's base is the journal ON DISK, never the in-memory
/// copy: under the write lock the journal is replayed and folded, the
/// `decide` closure mutates that fold and names the fact it caused,
/// and the re-fold — journal plus the appended record — is what is
/// BOTH written to disk and swapped into `state.index`, so memory and
/// catalog can never disagree about where they came from. `decide`
/// returning no event is a no-op (F2-3): a mutation that changes
/// nothing writes no fact, no catalog and no commit.
///
/// Returns `(stood, changed)`: `stood` — did the target stand in the
/// projection before the event (upsert inverts it into `created`; the
/// deletes report it as `removed`); `changed` — was an event applied
/// at all.
async fn mutate<F>(state: &AppState, decide: F) -> Result<(bool, bool), ApiError>
where
    F: FnOnce(&mut Index) -> (bool, Option<Event>),
{
    let mut idx = state.index.write().await;
    let journal_dir = default_dir(&state.data_dir);
    let mut records =
        replay(&journal_dir).map_err(|e| journal_refused("could not read the journal", e))?;
    let mut probe = project(records.iter().cloned())
        .map_err(|e| journal_refused("could not fold the journal into a catalog", e))?;
    let (stood, event) = decide(&mut probe);
    let Some(event) = event else {
        // F2-3 — nothing changed: no record, no write, no publish.
        return Ok((stood, false));
    };
    // F2-1 — the clock enters at the mutation event, once: the same
    // `at` stamps the record and the catalog it projects to.
    let at = Utc::now();
    let record = JournalRecord {
        at,
        actor: default_generator(),
        event,
    };
    append(&journal_dir, &record)
        .map_err(|e| journal_refused("could not append the fact to the journal", e))?;
    records.push(record);
    let fresh = project(records)
        .map_err(|e| journal_refused("could not fold the journal into a catalog", e))?;
    // Truth first: the record is already durable, so a failed
    // `write_to` leaves a journal whose next fold rebuilds the catalog
    // — the mutation is recoverable, never lost. Memory takes the
    // re-fold either way, so reads never serve a state the truth
    // layer cannot reproduce.
    let persisted = fresh.write_to(&state.data_dir, &WriteCtx { at });
    *idx = fresh;
    persisted.map_err(|e| {
        ApiError::internal(format!(
            "could not persist index: {e} — the fact is durable in the journal and the next mutation re-projects the catalog from it \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
              fix: check the data dir is writable, then retry)"
        ))
    })?;
    Ok((stood, true))
}

/// Map a truth-layer failure (the journal could not be read, folded,
/// or appended to) onto the API's 500: the inner detail is kept raw,
/// but its CLI-oriented recipe is replaced by the server's — the
/// catalog is never this writer's input, so the mutation did not
/// happen and nothing derived was touched.
fn journal_refused(what: &str, e: Error) -> ApiError {
    let detail = match e {
        Error::Io { path, message } => format!("`{}`: {message}", path.display()),
        Error::Malformed(m) | Error::Unprojectable(m) => m,
        other => other.to_string(),
    };
    ApiError::internal(format!(
        "{what}: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; \
          fix: the journal is the truth layer — restore the shard named above and retry; \
          the catalog was not touched and the mutation did not happen)"
    ))
}

fn require_writeable(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    if state.read_only {
        return Err(ApiError::forbidden(
            "server is running in --read-only mode \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
             fix: restart the server without --read-only to accept writes)",
        ));
    }
    if !state.tokens.has_any() {
        return Err(ApiError::forbidden(
            "server has no admin tokens configured \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
             fix: start the server with --auth-tokens-file to enable writes)",
        ));
    }
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let Some(token) = supplied else {
        return Err(ApiError::unauthorized());
    };
    if !state.tokens.check(token) {
        return Err(ApiError::unauthorized());
    }
    Ok(())
}

/// After a successful mutation, commit + push the data directory when
/// `--auto-commit-push` is on. Runs on a blocking thread under the
/// publish lock (serialised across mutations); a publish failure is
/// logged at `warn` and counted, but never fails the request (Р4) — the
/// mutation already succeeded. A no-op when the flag is off, so the
/// flag-off server never runs git (Р7).
async fn publish_mutation(state: &AppState, message: String) {
    if !state.auto_commit_push {
        return;
    }
    // Hold the publish lock across the blocking git work so two
    // concurrent commits never interleave in the one working copy.
    let joined = {
        let _guard = state.publish_lock.lock().await;
        let dir = state.data_dir.clone();
        tokio::task::spawn_blocking(move || crate::publish::commit_and_push(&dir, &message)).await
    };
    match joined {
        Ok(Ok(crate::publish::PublishOutcome::Published)) => {}
        Ok(Ok(crate::publish::PublishOutcome::NothingToCommit)) => {
            // Р6: a concurrent publish already shipped this change.
        }
        Ok(Err(e)) => {
            tracing::warn!(
                error = %e,
                "auto-commit-push failed after a successful mutation; \
                 the write stands and the index retries on the next mutation"
            );
            state.stats.note_publish_failure();
        }
        Err(e) => {
            tracing::warn!(error = %e, "auto-commit-push task join failed");
            state.stats.note_publish_failure();
        }
    }
}
