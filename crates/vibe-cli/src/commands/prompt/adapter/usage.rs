//! Cache diagnostics, read out of a provider's own usage record.
//!
//! **There is no constructor that takes a local estimate, and that is a
//! type-level guarantee rather than a comment.** `CacheDiagnostics`'s fields
//! are private and its only constructor takes a [`ProviderUsage`];
//! `ProviderUsage`'s fields are private and its only constructor parses a
//! provider response. So the shortest path from a guess to a diagnostic
//! goes through a compiler error (`SP-FMT-011`).
//!
//! **A number the provider did not report is absent, never zero and never
//! derived.** Zero is a measurement — "nothing was read from cache" — and a
//! missing counter is not that statement. Subtraction is worse: OpenAI
//! reports a cached-token count inside a prompt total, and computing
//! `total − cached` would publish an arithmetic result in a field whose
//! whole contract is that a provider said it. So the uncached count appears
//! for a provider that reports one and is absent for a provider that does
//! not.
//!
//! **This is not a cache claim.** A reading says what one response reported.
//! It is not a hit rate, not a TTL, not a saving, and not transferable to
//! another model, provider or day; the economics stay parametric and
//! unsubstituted (`#SYS-CACHE-FACTS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CACHE-FACTS");

use std::collections::BTreeMap;

use anyhow::{Result, bail};

/// One provider usage record, as the provider wrote it.
///
/// Construct it only by reading a response: there is deliberately no way to
/// assemble one from numbers a caller chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderUsage {
    /// Exactly the counters the record carried, by their own key names.
    counters: BTreeMap<String, u64>,
}

/// The counter names, per provider spelling: the cache-write count, the
/// cache-read count, and the uncached input count when the provider reports
/// one of its own.
///
/// Both live spellings are here because both are real and neither is a
/// superset: Anthropic's Messages API reports an `input_tokens` that already
/// excludes the cached part, while an OpenAI-compatible `usage` reports a
/// prompt total with the cached part inside it — which is exactly why the
/// uncached slot is `None` for the second one.
const SPELLINGS: [(&str, &str, &str, Option<&str>); 2] = [
    (
        "anthropic-messages-usage",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        Some("input_tokens"),
    ),
    (
        "openai-compatible-usage",
        "cache_write_input_tokens",
        "cached_input_tokens",
        None,
    ),
];

impl ProviderUsage {
    /// Read a usage record out of one provider response.
    ///
    /// `body` is the response document a caller already received; the
    /// `usage` object may be at the top level or nested one level down,
    /// which covers the shapes both live spellings use. A response with no
    /// usage object at all is a refusal: a diagnostics report built from
    /// nothing would be indistinguishable from one built from zeros.
    pub(crate) fn read(body: &str) -> Result<Self> {
        let document: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| anyhow::anyhow!("the provider response does not parse: {error}"))?;
        let Some(usage) = find_usage(&document) else {
            bail!(
                "the provider response carries no `usage` object, so there is nothing to read; \
                 cache diagnostics are read from a provider and never computed here"
            )
        };
        let mut counters = BTreeMap::new();
        for (key, value) in usage {
            if let Some(number) = value.as_u64() {
                counters.insert(key.clone(), number);
            }
        }
        // `prompt_tokens_details.cached_tokens` is the OpenAI chat spelling
        // of the read counter, one level down. Lifting it by its own name
        // keeps the map a record of what the provider wrote.
        if let Some(cached) = usage
            .get("prompt_tokens_details")
            .and_then(serde_json::Value::as_object)
            .and_then(|details| details.get("cached_tokens"))
            .and_then(serde_json::Value::as_u64)
        {
            counters.insert("cached_input_tokens".to_owned(), cached);
        }
        Ok(Self { counters })
    }

    /// The counter names the record carried, in order.
    ///
    /// Reported when none of them is a cache counter this build knows: the
    /// useful sentence for a caller holding a response that produced no
    /// diagnostics is *which* numbers their provider did send, not a
    /// restatement that the cache ones are missing.
    pub(crate) fn counter_names(&self) -> Vec<&str> {
        self.counters.keys().map(String::as_str).collect()
    }
}

/// The three counters, each present only because a provider reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheDiagnostics {
    source: &'static str,
    write_tokens: Option<u64>,
    read_tokens: Option<u64>,
    uncached_tokens: Option<u64>,
}

impl CacheDiagnostics {
    /// The **only** constructor, and it takes a provider's record.
    ///
    /// `None` when the record carries neither cache counter: a response from
    /// a provider that reports no caching at all yields no diagnostics,
    /// rather than a report of three zeros that would read as a measured
    /// miss.
    pub(crate) fn from_usage(usage: &ProviderUsage) -> Option<Self> {
        let (source, write, read, uncached) = SPELLINGS
            .iter()
            .find(|(_, write, read, _)| {
                usage.counters.contains_key(*write) || usage.counters.contains_key(*read)
            })
            .copied()?;
        Some(Self {
            source,
            write_tokens: usage.counters.get(write).copied(),
            read_tokens: usage.counters.get(read).copied(),
            uncached_tokens: uncached.and_then(|key| usage.counters.get(key).copied()),
        })
    }

    /// Which provider spelling the numbers came from.
    pub(crate) fn source(&self) -> &'static str {
        self.source
    }

    /// Tokens the provider reported writing into its cache.
    pub(crate) fn write_tokens(&self) -> Option<u64> {
        self.write_tokens
    }

    /// Tokens the provider reported reading from its cache.
    pub(crate) fn read_tokens(&self) -> Option<u64> {
        self.read_tokens
    }

    /// Input tokens the provider itself reported as uncached, for a
    /// provider that reports such a number. Never a subtraction.
    pub(crate) fn uncached_tokens(&self) -> Option<u64> {
        self.uncached_tokens
    }
}

/// The `usage` object, at the top level or one level down.
fn find_usage(document: &serde_json::Value) -> Option<&serde_json::Map<String, serde_json::Value>> {
    if let Some(usage) = document.get("usage").and_then(serde_json::Value::as_object) {
        return Some(usage);
    }
    document
        .as_object()?
        .values()
        .filter_map(serde_json::Value::as_object)
        .find_map(|nested| nested.get("usage").and_then(serde_json::Value::as_object))
}

#[cfg(test)]
#[path = "usage/tests.rs"]
mod tests;
