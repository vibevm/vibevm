//! The channel table an activation reads, from the template cache alone
//! (PROP-075 `#CHANNEL-SYNTAX`, `#ACTIVATION-HOOK`).
//!
//! `vibe app env`, `vibe app exec` and the prompt hook that calls the
//! first of them resolve a channel named in a `--use`, a project
//! requirement or a version file. The budget `#ACTIVATION-HOOK` sets for
//! that is hard: **no network, no registry call and no index-server
//! read**. So the answer is read from the template layer's own cache —
//! the declaration kept beside the inputs plus the cached version list,
//! two small file reads — and a package with nothing cached simply has no
//! channel, which the activation reports as `unresolved_channel` rather
//! than as an error. A prompt hook must never fail a `cd`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK");

use std::collections::BTreeMap;
use std::sync::Mutex;

use vibe_app::ChannelMembership;
use vibe_core::Group;

/// One package's channel table: channel name → its members.
type Table = BTreeMap<String, Vec<semver::Version>>;

/// The cached channel table, memoised per package for the invocation so a
/// multi-slot activation reads each package's cache once. An inner `None`
/// is a package this invocation looked for and found nothing cached about
/// — remembered, so a second slot does not look again.
#[derive(Default)]
pub struct CachedChannels {
    tables: Mutex<BTreeMap<String, Option<Table>>>,
}

impl CachedChannels {
    /// An empty table, filled lazily from the template cache.
    pub fn new() -> Self {
        CachedChannels::default()
    }
}

impl ChannelMembership for CachedChannels {
    fn members(&self, package: &str, channel: &str) -> Option<Vec<semver::Version>> {
        let mut tables = self.tables.lock().ok()?;
        let table = tables
            .entry(package.to_string())
            .or_insert_with(|| table_of(package));
        table.as_ref().and_then(|table| table.get(channel).cloned())
    }
}

/// One package's channel table, read from the cache alone. `None` when
/// this invocation installed no template layer, when the coordinate is
/// not a `group/name`, or when nothing about that package is cached.
fn table_of(package: &str) -> Option<Table> {
    let layer = vibe_registry::templates::active()?;
    let (group, name) = package.split_once('/')?;
    let group = Group::parse(group).ok()?;
    let table = layer.cached_channels(&group, name)?;
    Some(
        table
            .into_iter()
            .map(|(channel, members)| (channel, members.into_iter().collect()))
            .collect(),
    )
}
