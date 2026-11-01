//! One home's remap state, read and written in one place
//! (`PROP-066#consent`, `PROP-066#providers`).
//!
//! Three files live under `<home>/remap/`: the installed packages, the
//! acceptance ledger, and the derived provider map. Every verb that
//! touches any of them touches all three — accepting a package changes
//! which rows are live, which changes the map — so they are loaded and
//! saved as one bundle under one lock rather than three times per command.
//!
//! The map is derived and is rebuilt on every save, which is what keeps
//! `PROP-066#providers`' "never the sole home of a fact" true in code
//! rather than only in prose.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#providers");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use vibe_core::remap::{
    ConsentLedger, HomeLock, InstalledRemap, InstalledRemaps, OfferState, ProviderMap, RemapPackage,
};

/// The remap state of one home, read together.
pub(crate) struct RemapHome {
    /// The user settings root (or a project home) the files live under.
    root: PathBuf,
    /// `<home>/remap/installed.json`.
    pub(crate) installed: InstalledRemaps,
    /// `<home>/remap/consent.json`.
    pub(crate) ledger: ConsentLedger,
}

impl RemapHome {
    /// The user settings root, resolved through the one chokepoint.
    pub(crate) fn user_root() -> Result<PathBuf> {
        vibe_core::settings::settings_dir().ok_or_else(|| {
            anyhow!(
                "cannot resolve the vibe settings home — set $HOME (or $VIBE_SETTINGS) so the \
                 remap acceptance ledger has a place to live \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-066#consent)"
            )
        })
    }

    /// Read the remap state of the user home.
    pub(crate) fn open() -> Result<Self> {
        Self::open_at(&Self::user_root()?)
    }

    /// Read the remap state of an explicit home — what a test and a
    /// second scratch home of `remap import` use.
    pub(crate) fn open_at(root: &Path) -> Result<Self> {
        Ok(RemapHome {
            root: root.to_path_buf(),
            installed: InstalledRemaps::load(root).with_context(|| {
                format!(
                    "reading the installed remap packages of `{}`",
                    root.display()
                )
            })?,
            ledger: ConsentLedger::load(root).with_context(|| {
                format!(
                    "reading the remap acceptance ledger of `{}`",
                    root.display()
                )
            })?,
        })
    }

    /// The home's root.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Take the home's mutation lock, so two sessions cannot rebuild the
    /// derived map at once.
    pub(crate) fn lock(&self) -> Result<HomeLock> {
        vibe_core::remap::lock(&self.root)
            .with_context(|| format!("locking the remap state of `{}`", self.root.display()))
    }

    /// The state of one installed package.
    pub(crate) fn state(&self, key: &str) -> Option<OfferState> {
        self.installed
            .find(key)
            .map(|entry| self.ledger.state(key, &entry.package.content_hash))
    }

    /// Every installed offer, in the order `PROP-066#package` ranks them
    /// (lowest `priority` first, so the highest wins the layer's
    /// later-row tie-break).
    pub(crate) fn offers(&self) -> Vec<RemapPackage> {
        self.installed.offers()
    }

    /// The offers whose rows currently apply.
    pub(crate) fn accepted(&self) -> Vec<RemapPackage> {
        self.offers()
            .into_iter()
            .filter(|offer| self.ledger.state(&offer.key, &offer.content_hash).applies())
            .collect()
    }

    /// The offers a run would have to ask about: installed, and neither
    /// accepted nor rejected (`PROP-066#consent`).
    pub(crate) fn pending(&self) -> Vec<RemapPackage> {
        self.offers()
            .into_iter()
            .filter(|offer| self.ledger.state(&offer.key, &offer.content_hash).asks())
            .collect()
    }

    /// The derived provider map as the files on disk imply it.
    pub(crate) fn providers(&self) -> ProviderMap {
        ProviderMap::derive(&self.offers(), &self.ledger)
    }

    /// Add or replace an installed package.
    pub(crate) fn install(&mut self, entry: InstalledRemap) {
        self.installed.upsert(entry);
    }

    /// Write every file of the home, rebuilding the derived map
    /// (`PROP-066#providers`). The caller holds [`Self::lock`].
    pub(crate) fn save(&self) -> Result<()> {
        self.installed
            .store(&self.root)
            .context("writing the installed remap packages")?;
        self.ledger
            .store(&self.root)
            .context("writing the remap acceptance ledger")?;
        self.providers()
            .store(&self.root)
            .context("writing the derived remap provider map")?;
        Ok(())
    }
}
