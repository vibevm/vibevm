//! Tariff-hygiene downloader (plan D12): workers have web tools denied, so
//! documents are fetched ONCE, locally, by the boss or a human.  This cell
//! is the only sanctioned network egress for document acquisition.

use crate::{EXIT_INFRA, EXIT_NEGATIVE, EXIT_OK, fail_code};

specmark::scope!("spec://fractality/PROP-001#architecture");

fn supported_url(url: &str) -> Result<(), String> {
    if let Some(scheme) = url.split_once(':').map(|(s, _)| s) {
        if scheme == "http" || scheme == "https" {
            return Ok(());
        }
        return Err(format!("unsupported scheme: {scheme}"));
    }
    Err(format!("no scheme in url: {url}"))
}

pub(crate) async fn fetch(url: &str, out: &camino::Utf8Path, force: bool) -> u8 {
    if let Err(e) = supported_url(url) {
        return fail_code(EXIT_NEGATIVE, &e);
    }

    if out.exists() && !force {
        return fail_code(
            EXIT_NEGATIVE,
            &format!("{out} already exists; use --force to overwrite"),
        );
    }

    if let Some(parent) = out.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return fail_code(
            EXIT_INFRA,
            &format!("creating parent directory `{parent}`: {e}"),
        );
    }

    let client = reqwest::Client::new();
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => return fail_code(EXIT_INFRA, &format!("fetching {url}: {e}")),
    };

    let status = response.status();
    if !status.is_success() {
        return fail_code(EXIT_NEGATIVE, &format!("fetch {url}: HTTP {status}"));
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => return fail_code(EXIT_INFRA, &format!("reading body from {url}: {e}")),
    };

    match std::fs::write(out.as_std_path(), &bytes) {
        Ok(()) => {
            println!("fetched {url} -> {out} ({} bytes)", bytes.len());
            EXIT_OK
        }
        Err(e) => fail_code(EXIT_INFRA, &format!("writing {out}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_url_ok() {
        assert!(supported_url("https://example.com/doc.md").is_ok());
    }

    #[test]
    fn http_url_ok() {
        assert!(supported_url("http://example.com/doc.md").is_ok());
    }

    #[test]
    fn file_scheme_rejected() {
        let err = supported_url("file:///etc/passwd").unwrap_err();
        assert!(err.contains("file"));
    }

    #[test]
    fn ftp_scheme_rejected() {
        let err = supported_url("ftp://files.example.com/data.bin").unwrap_err();
        assert!(err.contains("ftp"));
    }

    #[test]
    fn no_scheme_rejected() {
        let err = supported_url("example.com/doc.md").unwrap_err();
        assert!(err.contains("no scheme"));
    }
}
