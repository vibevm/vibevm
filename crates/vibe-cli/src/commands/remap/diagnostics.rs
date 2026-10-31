//! What a reader needs to judge a redirection, computed here and never
//! trusted from the row (`PROP-066#diagnostics`).
//!
//! Being sent to another address is suspicious by nature, so every view of
//! a row states the scheme of its target, whether the host is private or
//! public, and the three things that deserve a second look: a plaintext
//! target, a `prefix` row over a public host, and a target this run could
//! not classify. A row whose target cannot be classified is shown as
//! `unknown`, never hidden.
//!
//! Classification is deliberately cheap and deliberately honest. An
//! address literal is decided arithmetically. A name is decided by one DNS
//! lookup — skipped entirely under `--offline`, where the honest answer is
//! `unknown` rather than a guess, and skipped for a host that resolves to
//! nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#diagnostics");

use std::net::{IpAddr, ToSocketAddrs};

use vibe_core::remap::{RemapKey, RemapTarget};
use vibe_wire::generated::remap_listing as wire;

/// The computed diagnostics of one target.
pub(super) struct Diagnosis {
    /// `https`, `http`, `file`, `path`, `ssh`, `git`, `deny`, or the
    /// scheme as written.
    pub scheme: String,
    /// Where the target lives.
    pub reach: wire::Reach,
    /// The host, when the target names one.
    pub host: Option<String>,
    /// `unencrypted`, `broad`, `unknown`.
    pub notes: Vec<wire::Note>,
}

/// Classify one row's target. `offline` suppresses every name lookup, so
/// `remap list --offline` touches no network at all.
pub(super) fn diagnose(key: &RemapKey, target: &RemapTarget, offline: bool) -> Diagnosis {
    let (scheme, host) = match target {
        RemapTarget::Deny => ("deny".to_owned(), None),
        RemapTarget::Path(_) => ("path".to_owned(), None),
        RemapTarget::Url(url) => split_url(url),
    };
    let reach = match (&scheme[..], host.as_deref()) {
        // A refusal has no target to reach, and a path and a `file://`
        // URL are the local disk by construction.
        ("deny", _) | ("path", _) | ("file", _) => wire::Reach::Local,
        (_, Some(host)) => classify_host(host, offline),
        // A URL with no authority — a scheme we do not parse, or a
        // relative spelling — is exactly the case `#diagnostics` says to
        // show rather than hide.
        (_, None) => wire::Reach::Unknown,
    };
    let mut notes = Vec::new();
    if scheme == "http" {
        notes.push(wire::Note::Unencrypted);
    }
    // A `prefix` row over a public host rewrites every URL under it, which
    // is the widest thing a row can do and the one the reader most needs
    // pointed out.
    if matches!(key, RemapKey::Prefix(_)) && reach == wire::Reach::Public {
        notes.push(wire::Note::Broad);
    }
    if reach == wire::Reach::Unknown {
        notes.push(wire::Note::Unknown);
    }
    Diagnosis {
        scheme,
        reach,
        host,
        notes,
    }
}

/// Count the rows pointing at each host, for the footer of
/// `PROP-066#display`. A target with no host — a path, a `file://` URL —
/// is grouped under `local path`, which is how the footer's example reads.
pub(super) fn host_summary(
    rows: &[(String, wire::Reach, Option<String>)],
) -> Vec<wire::HostSummary> {
    let mut summary: Vec<wire::HostSummary> = Vec::new();
    for (scheme, reach, host) in rows {
        if scheme == "deny" {
            continue;
        }
        let name = host.clone().unwrap_or_else(|| "local path".to_owned());
        match summary.iter_mut().find(|entry| entry.host == name) {
            Some(entry) => entry.rows += 1,
            None => summary.push(wire::HostSummary {
                host: name,
                rows: 1,
                scheme: scheme.clone(),
                reach: reach.clone(),
            }),
        }
    }
    summary
}

/// The scheme and host of a URL, for the shapes a remap target takes: an
/// `https://` / `http://` / `file://` URL, an `ssh://` or `git://` one, and
/// the scp-form `git@host:path` git accepts.
fn split_url(url: &str) -> (String, Option<String>) {
    let url = url.trim();
    if let Some((scheme, rest)) = url.split_once("://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_userinfo, host)| host);
        // A port is not part of the host for classification; a `:` with
        // anything else after it belongs to an IPv6 literal.
        let host = match host.rsplit_once(':') {
            Some((head, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
                head
            }
            _ => host,
        };
        let host = host.trim_matches(['[', ']']);
        return (
            scheme.to_ascii_lowercase(),
            (!host.is_empty()).then(|| host.to_ascii_lowercase()),
        );
    }
    // scp-form `user@host:path` — ssh under the hood, so it is named by
    // its host the way `#diagnostics` asks.
    if let Some((userinfo, rest)) = url.split_once('@')
        && !userinfo.is_empty()
        && let Some((host, _path)) = rest.split_once(':')
        && !host.is_empty()
    {
        return ("ssh".to_owned(), Some(host.to_ascii_lowercase()));
    }
    (String::new(), None)
}

/// Private or public, by address literal when the host is one, else by one
/// DNS lookup. Under `--offline` no lookup is made and a name is
/// `unknown`: a classification nobody checked would be worse than an
/// admitted gap.
fn classify_host(host: &str, offline: bool) -> wire::Reach {
    if let Ok(address) = host.parse::<IpAddr>() {
        return reach_of(address);
    }
    if offline {
        return wire::Reach::Unknown;
    }
    // `localhost` and its kin resolve to a loopback literal, so the lookup
    // answers them too and no name is special-cased here.
    match (host, 0u16).to_socket_addrs() {
        Ok(mut addresses) => match addresses.next() {
            Some(address) => reach_of(address.ip()),
            None => wire::Reach::Unknown,
        },
        Err(_) => wire::Reach::Unknown,
    }
}

/// Whether one address is reachable only inside a network. Loopback,
/// link-local, the three RFC-1918 ranges, the shared-address space and
/// unique-local IPv6 are private; everything else routable is public.
fn reach_of(address: IpAddr) -> wire::Reach {
    match address {
        IpAddr::V4(v4) => {
            if v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified() {
                wire::Reach::Private
            } else if v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]) {
                // RFC 6598 carrier-grade NAT — not the public internet.
                wire::Reach::Private
            } else {
                wire::Reach::Public
            }
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            if v6.is_loopback() || v6.is_unspecified() {
                wire::Reach::Private
            } else if (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80 {
                // Unique-local (fc00::/7) and link-local (fe80::/10).
                wire::Reach::Private
            } else {
                wire::Reach::Public
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn an_address_literal_is_classified_without_any_lookup() {
        let prefix = RemapKey::Prefix("https://github.com/".into());
        for (url, reach) in [
            ("https://10.1.2.3/a", wire::Reach::Private),
            ("https://192.168.0.9/a", wire::Reach::Private),
            ("https://127.0.0.1:8080/a", wire::Reach::Private),
            ("https://100.70.0.1/a", wire::Reach::Private),
            ("https://93.184.216.34/a", wire::Reach::Public),
            ("https://[fd00::1]/a", wire::Reach::Private),
            ("https://[2606:4700::1]/a", wire::Reach::Public),
        ] {
            let found = diagnose(&prefix, &RemapTarget::Url(url.into()), true);
            assert_eq!(found.reach, reach, "{url}");
            assert_eq!(found.scheme, "https", "{url}");
        }
    }

    #[test]
    fn a_path_and_a_file_url_are_local_and_a_deny_has_no_target() {
        let key = RemapKey::Sha256("ab".repeat(32));
        let path = diagnose(&key, &RemapTarget::Path(PathBuf::from("/srv/a")), true);
        assert_eq!(
            (path.scheme.as_str(), path.reach.clone()),
            ("path", wire::Reach::Local)
        );
        assert!(path.notes.is_empty());

        let file = diagnose(&key, &RemapTarget::Url("file:///srv/a".into()), true);
        assert_eq!(file.reach, wire::Reach::Local);

        let deny = diagnose(&key, &RemapTarget::Deny, true);
        assert_eq!((deny.scheme.as_str(), deny.host), ("deny", None));
    }

    #[test]
    fn plaintext_is_noted_and_a_broad_prefix_over_a_public_host_is_noted() {
        let prefix = RemapKey::Prefix("https://github.com/".into());
        let plain = diagnose(&prefix, &RemapTarget::Url("http://10.0.0.1/a".into()), true);
        assert_eq!(plain.notes, [wire::Note::Unencrypted]);

        let broad = diagnose(
            &prefix,
            &RemapTarget::Url("https://93.184.216.34/".into()),
            true,
        );
        assert_eq!(broad.notes, [wire::Note::Broad]);

        // The same public target under a `sha256` key is one artifact, not
        // a whole host, so it is not broad.
        let narrow = diagnose(
            &RemapKey::Sha256("ab".repeat(32)),
            &RemapTarget::Url("https://93.184.216.34/".into()),
            true,
        );
        assert!(narrow.notes.is_empty());
    }

    #[test]
    fn a_name_is_unknown_offline_and_an_unparseable_target_always_is() {
        let key = RemapKey::Registry("vibespecs".into());
        let name = diagnose(
            &key,
            &RemapTarget::Url("https://nexus.corp.example/v".into()),
            true,
        );
        assert_eq!(name.reach, wire::Reach::Unknown);
        assert_eq!(name.host.as_deref(), Some("nexus.corp.example"));
        assert_eq!(name.notes, [wire::Note::Unknown]);

        let nonsense = diagnose(&key, &RemapTarget::Url("not-a-url".into()), true);
        assert_eq!(nonsense.reach, wire::Reach::Unknown);
        assert!(nonsense.host.is_none());
    }

    #[test]
    fn scp_form_git_urls_are_named_by_their_host() {
        let key = RemapKey::Registry("internal".into());
        let found = diagnose(
            &key,
            &RemapTarget::Url("git@git.corp.example:org/repo".into()),
            true,
        );
        assert_eq!(found.scheme, "ssh");
        assert_eq!(found.host.as_deref(), Some("git.corp.example"));
    }

    #[test]
    fn the_footer_counts_rows_per_host_and_leaves_denies_out() {
        let rows = vec![
            (
                "https".to_owned(),
                wire::Reach::Private,
                Some("nexus".to_owned()),
            ),
            (
                "https".to_owned(),
                wire::Reach::Private,
                Some("nexus".to_owned()),
            ),
            ("path".to_owned(), wire::Reach::Local, None),
            ("deny".to_owned(), wire::Reach::Local, None),
        ];
        let summary = host_summary(&rows);
        assert_eq!(summary.len(), 2);
        assert_eq!((summary[0].host.as_str(), summary[0].rows), ("nexus", 2));
        assert_eq!(
            (summary[1].host.as_str(), summary[1].rows),
            ("local path", 1)
        );
    }
}
