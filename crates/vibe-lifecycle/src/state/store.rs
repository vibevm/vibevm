//! Strict generated-type state reader and record-last atomic writer.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use specmark::spec;
use thiserror::Error;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordStatus, LifecycleState, StateRun,
};

const SCHEMA: u32 = 1;

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML")]
pub enum LifecycleStateError {
    #[error(
        "cannot read lifecycle state `{path}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "malformed lifecycle state `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Malformed { path: PathBuf, reason: String },
    #[error(
        "unsupported lifecycle state schema {schema} in `{path}`; this build supports schema 1 \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Unsupported { path: PathBuf, schema: u32 },
    #[error(
        "cannot write lifecycle state `{path}` atomically: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: ensure `.vibe/` is writable and rerun)"
    )]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "cannot encode lifecycle state `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: report this generated-wire serialization failure)"
    )]
    Encode { path: PathBuf, reason: String },
}

/// Open current state, replace the whole-run header, preserve every old row,
/// and immediately checkpoint the initial record (even for an empty plan).
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML")]
pub struct LifecycleStateStore {
    path: PathBuf,
    state: LifecycleState,
}

impl LifecycleStateStore {
    pub const FILE: &'static str = ".vibe/lifecycle.toml";

    pub fn begin(
        workspace_root: &Path,
        requested: String,
        chain: Vec<String>,
        started: String,
    ) -> Result<Self, LifecycleStateError> {
        let path = workspace_root.join(Self::FILE);
        let execution = match fs::read_to_string(&path) {
            Ok(text) => {
                let previous: LifecycleState =
                    toml::from_str(&text).map_err(|error| LifecycleStateError::Malformed {
                        path: path.clone(),
                        reason: error.to_string(),
                    })?;
                if previous.schema != SCHEMA {
                    return Err(LifecycleStateError::Unsupported {
                        path,
                        schema: previous.schema,
                    });
                }
                previous.execution
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(source) => {
                return Err(LifecycleStateError::Read {
                    path: path.clone(),
                    source,
                });
            }
        };
        let store = Self {
            path,
            state: LifecycleState {
                execution,
                run: StateRun {
                    chain,
                    requested,
                    started,
                },
                schema: SCHEMA,
            },
        };
        store.write()?;
        Ok(store)
    }

    #[must_use]
    pub fn prior(&self, key: &str) -> Option<&ExecutionRecord> {
        self.state.execution.get(key)
    }

    #[must_use]
    pub fn reusable(&self, key: &str, fingerprint: &str) -> bool {
        self.reusable_record(key, fingerprint).is_some()
    }

    #[must_use]
    pub fn reusable_record(&self, key: &str, fingerprint: &str) -> Option<&ExecutionRecord> {
        self.prior(key).filter(|record| {
            record.fingerprint == fingerprint
                && matches!(
                    record.status,
                    ExecutionRecordStatus::Ok
                        | ExecutionRecordStatus::Skip
                        | ExecutionRecordStatus::Fresh
                )
        })
    }

    pub fn checkpoint(
        &mut self,
        key: String,
        record: ExecutionRecord,
    ) -> Result<(), LifecycleStateError> {
        self.state.execution.insert(key, record);
        self.write()
    }

    /// Prune vanished synthetic rows only after their owner has reconciled
    /// durable outputs successfully. Other lifecycle history is untouched.
    pub fn retain_prefixed(
        &mut self,
        prefix: &str,
        keep: &BTreeSet<String>,
    ) -> Result<(), LifecycleStateError> {
        let before = self.state.execution.len();
        self.state
            .execution
            .retain(|key, _| !key.starts_with(prefix) || keep.contains(key));
        if self.state.execution.len() != before {
            self.write()?;
        }
        Ok(())
    }

    #[must_use]
    pub fn state(&self) -> &LifecycleState {
        &self.state
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn write(&self) -> Result<(), LifecycleStateError> {
        let bytes = toml::to_string_pretty(&self.state)
            .map_err(|error| LifecycleStateError::Encode {
                path: self.path.clone(),
                reason: error.to_string(),
            })?
            .into_bytes();
        let Some(parent) = self.path.parent() else {
            return Err(LifecycleStateError::Write {
                path: self.path.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "lifecycle state path has no parent",
                ),
            });
        };
        fs::create_dir_all(parent).map_err(|source| LifecycleStateError::Write {
            path: self.path.clone(),
            source,
        })?;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let tmp = parent.join(format!(
            ".lifecycle.toml.tmp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let result = (|| {
            let mut file = File::create(&tmp)?;
            file.write_all(&bytes)?;
            file.flush()?;
            file.sync_all()?;
            fs::rename(&tmp, &self.path)?;
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
            Ok::<_, std::io::Error>(())
        })();
        if let Err(source) = result {
            let _ = fs::remove_file(&tmp);
            return Err(LifecycleStateError::Write {
                path: self.path.clone(),
                source,
            });
        }
        Ok(())
    }
}
