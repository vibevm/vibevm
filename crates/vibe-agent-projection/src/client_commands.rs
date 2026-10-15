//! Closed client recipes. Parameters are data; this module has no process port.
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Claude,
    Codex,
    OpenCode,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Version,
    List,
    MarketplaceAdd(PathBuf),
    Install(String),
    Remove(String),
}
impl Operation {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::List => "list",
            Self::MarketplaceAdd(_) => "marketplace_add",
            Self::Install(_) => "install",
            Self::Remove(_) => "remove",
        }
    }
}
pub fn supported_version(client: Client, version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    let pair = match client {
        Client::Claude => ("2", "1"),
        Client::Codex => ("0", "148"),
        Client::OpenCode => ("1", "17"),
    };
    parts.len() == 3
        && parts[0] == pair.0
        && parts[1] == pair.1
        && !parts[2].is_empty()
        && parts[2].bytes().all(|b| b.is_ascii_digit())
        && parts[2].parse::<u64>().is_ok()
}
/// Native path arguments remain OsString, including non-UTF8 paths. A wire
/// projection must separately refuse paths it cannot represent losslessly.
pub fn render(client: Client, operation: &Operation) -> Result<Vec<OsString>, &'static str> {
    use Client::*;
    use Operation::*;
    let fixed: &[&str] = match operation {
        Version => &["--version"],
        _ if client == OpenCode => return Err("unsupported client operation"),
        List => &["plugin", "list", "--json"],
        MarketplaceAdd(path) => {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
                || path.as_os_str().as_encoded_bytes().contains(&0)
            {
                return Err("marketplace path must be absolute and normalized");
            }
            if client == Claude {
                &["plugin", "marketplace", "add", "--scope", "user"]
            } else {
                &["plugin", "marketplace", "add", "--json"]
            }
        }
        Install(value) | Remove(value) => {
            if value.is_empty()
                || value.starts_with('-')
                || value.chars().any(char::is_control)
                || value.len() > 4096
            {
                return Err("invalid coordinate");
            }
            match (client, operation) {
                (Claude, Install(_)) => &["plugin", "install", "--scope", "user"],
                (Claude, _) => &["plugin", "uninstall", "--scope", "user"],
                (_, Install(_)) => &["plugin", "add", "--json"],
                _ => &["plugin", "remove", "--json"],
            }
        }
    };
    let mut args: Vec<OsString> = fixed.iter().map(OsString::from).collect();
    match operation {
        MarketplaceAdd(p) => args.push(p.as_os_str().into()),
        Install(s) | Remove(s) => args.push(s.into()),
        _ => {}
    }
    Ok(args)
}
#[cfg(test)]
mod m_08_d_1_cases;
