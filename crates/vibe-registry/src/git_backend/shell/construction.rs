//! Backend construction keeps transport bindings explicit before first use.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend");
use super::*;

impl ShellGit {
    /// Construct a [`ShellGit`] bound to `git` on `PATH`.
    ///
    /// Does **not** preflight — the cost is deferred to the first real
    /// operation, which fails fast with [`GitError::NotInstalled`] if
    /// git cannot be spawned. Call [`ShellGit::preflight`] once per
    /// `vibe` invocation to turn that into an up-front error.
    pub fn new() -> Self {
        let binary = Self::configured_binary();
        ShellGit {
            binary,
            admission: None,
            native: None,
            native_registry_auth: None,
            force_anonymous: false,
            raw_base: None,
            raw_client: OnceLock::new(),
            raw_sleeper: Sleeper::default(),
            preflight_cache: OnceLock::new(),
            transfers: Arc::default(),
        }
    }

    /// Bind HTTP admission before use and discard any prior HTTP pool.
    /// Native Git is unavailable under this temporary HTTP-only capability.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW"
    )]
    pub fn with_http_admission(mut self, admission: Arc<crate::HttpAdmission>) -> Self {
        self.native = None;
        self.native_registry_auth = None;
        self.admission = Some(admission);
        self.raw_client = OnceLock::new();
        self
    }

    /// Bind direct HTTPS preparation with its explicit trusted installation.
    /// Legacy execution remains closed; ordinary `with_binary` has no authority
    /// over this separately captured installation.
    pub fn with_native_admission(
        mut self,
        profile: Arc<native_admission::NativeGitAdmission>,
    ) -> Self {
        self.admission = Some(profile.http_admission());
        self.native_registry_auth = None;
        self.native = Some(Arc::new(native_admission::NativeState::new(profile)));
        self.raw_client = OnceLock::new();
        self
    }

    /// Rebind the `git` binary this backend spawns, overriding both
    /// `PATH` and `VIBE_GIT_BINARY`.
    ///
    /// Clears the cached preflight result — the new binary has not been
    /// probed, and answering for it from the old one's probe would be a
    /// lie in either direction.
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self.preflight_cache = OnceLock::new();
        self
    }

    /// Rebind the base the HTTPS read fast path reads from — for every
    /// host it knows, so one local server can stand in for all of them.
    ///
    /// Exists for tests. The crate's own precedent for this is
    /// `GitPerPackageRegistry::open_with_explicit_token`: an explicit
    /// constructor argument rather than an environment variable, because
    /// `#![forbid(unsafe_code)]` bars a test from mutating the process
    /// environment on Rust 2024, and a process-global would in any case
    /// be shared by every test running in parallel in the same binary.
    pub fn with_raw_base(mut self, raw_base: impl Into<String>) -> Self {
        self.raw_base = Some(raw_base.into());
        self
    }

    /// Rebind where the read fast path's pauses between retries go.
    ///
    /// Exists for tests, by the same reasoning as [`with_raw_base`] and
    /// with one more of its own: a suite that asserts a backoff schedule
    /// by sleeping through it is a suite that gets slower every time the
    /// schedule grows, and slow tests are how a schedule stops being
    /// asserted at all. The pause the read decided on is handed here
    /// instead, where a test can record it and return at once.
    ///
    /// [`with_raw_base`]: ShellGit::with_raw_base
    pub fn with_raw_sleeper(mut self, sleeper: impl Fn(Duration) + Send + Sync + 'static) -> Self {
        self.raw_sleeper = Sleeper(Arc::new(sleeper));
        self
    }
    // Preserve immutable capability/state sharing without exposing a general
    // Clone implementation or a mutable auth slot on the shared backend.
    pub(super) fn capability_clone(&self) -> Self {
        Self {
            binary: self.binary.clone(),
            admission: self.admission.clone(),
            native: self.native.clone(),
            native_registry_auth: self.native_registry_auth,
            force_anonymous: self.force_anonymous,
            raw_base: self.raw_base.clone(),
            raw_client: OnceLock::from(self.raw_client().cloned()),
            raw_sleeper: self.raw_sleeper.clone(),
            preflight_cache: OnceLock::new(),
            transfers: Arc::clone(&self.transfers),
        }
    }
    pub(super) fn registry_auth_clone(
        &self,
        auth: vibe_core::manifest::AuthKind,
    ) -> Result<Option<Arc<dyn GitBackend>>, GitError> {
        let Some(native) = &self.native else {
            return Ok(None);
        };
        native.profile.preflight_registry_auth(auth)?;
        if (self.force_anonymous || self.native_registry_auth.is_some_and(|bound| bound != auth))
            && !matches!(auth, vibe_core::manifest::AuthKind::None)
        {
            return Err(native_admission::unsupported(
                "a registry-bound or anonymous capability cannot gain a different credential mechanism",
            ));
        }
        let mut cloned = self.capability_clone();
        cloned.native_registry_auth = Some(auth);
        cloned.force_anonymous |= matches!(auth, vibe_core::manifest::AuthKind::None);
        Ok(Some(Arc::new(cloned)))
    }
}
