//! Compiled fake clients and the isolated engine world they run inside.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use vibe_core::manifest::{ArtifactKind, DeployTarget, MechanismRoutes};
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::PluginClient;
use crate::mechanism::deploy::model::{
    ClientExecutable, ClientExecutables, DeployExecution, DeploySelection, deploy_state_home,
};
use crate::mechanism::package::support::{config, empty_world, key, registry, temp};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};

pub(crate) struct World {
    pub(crate) project: TempDir,
    pub(crate) settings: TempDir,
    pub(crate) home: TempDir,
    pub(crate) fake: TempDir,
    pub(crate) state_home: PathBuf,
    pub(crate) clients: ClientExecutables,
    pub(crate) registry: vibe_extension_registry::MechanismRegistry,
    pub(crate) routes: MechanismRoutes,
}

impl World {
    pub(crate) fn new() -> Self {
        let project = temp();
        let settings = temp();
        let home = temp();
        let fake = temp();
        let clients = compile_clients(fake.path());
        Self {
            state_home: deploy_state_home(settings.path()),
            project,
            settings,
            home,
            fake,
            clients,
            registry: registry(&empty_world()),
            routes: MechanismRoutes::default(),
        }
    }

    pub(crate) fn record_projection(&self, id: &str, files: &[(&str, &str)]) -> String {
        let relative = format!("target/vibe-package/{id}");
        let root = self.project.path().join(&relative);
        for (name, bytes) in files {
            write(&root, name, bytes);
        }
        let tree = crate::mechanism::contain::tree_digest(&root).expect("fixture tree digests");
        let absolute = crate::mechanism::contain::forward_slashed(&root);
        let record = build_record(&RecordInputs {
            target: id,
            mechanism: &key(&format!(
                "package:{}-plugin",
                id.split('-').next().unwrap_or("claude")
            )),
            provider_key: "org.vibevm/vibe#fixture-projection",
            provider_version: None,
            provider_hash: None,
            output_id: id,
            kind: ArtifactKind::Directory,
            shape: ArtifactShape::Directory,
            digest: &tree.digest,
            path_absolute: &absolute,
            path_relative: &relative,
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: None,
            created_at: "2026-08-30T00:00:00Z",
            evidence: "client projection fixture".to_owned(),
        })
        .expect("projection record builds");
        write_record(self.project.path(), &record).expect("projection record writes");
        tree.digest
    }

    pub(crate) fn execution<'a>(
        &'a self,
        targets: &'a [DeployTarget],
        selection: &'a DeploySelection,
    ) -> DeployExecution<'a> {
        DeployExecution {
            project_root: self.project.path(),
            targets,
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home: &self.state_home,
            settings_root: self.settings.path(),
            user_home: self.home.path(),
            clients: &self.clients,
            project: "org.example/plugin-test",
            package: None,
            created_at: "2026-08-30T12:00:00Z",
        }
    }

    pub(crate) fn at(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.home.path().to_path_buf(), |path, part| path.join(part))
    }

    pub(crate) fn trace(&self) -> Vec<String> {
        std::fs::read_to_string(self.fake.path().join("fake-client.trace"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    pub(crate) fn home_census(&self) -> Vec<(String, String)> {
        fn descend(root: &Path, at: &Path, entries: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(at).expect("fixture census reads") {
                let path = entry.expect("fixture census entry").path();
                let relative = path
                    .strip_prefix(root)
                    .expect("fixture census remains contained")
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                let metadata = std::fs::symlink_metadata(&path).expect("fixture census metadata");
                if metadata.is_dir() {
                    entries.push((relative, "directory".to_owned()));
                    descend(root, &path, entries);
                } else {
                    let (digest, _) = crate::mechanism::contain::digest_file(&path)
                        .expect("fixture census hashes files");
                    entries.push((relative, digest));
                }
            }
        }

        let mut entries = Vec::new();
        descend(self.home.path(), self.home.path(), &mut entries);
        entries.sort();
        entries
    }

    pub(crate) fn set_claude_witness(&self, enabled: bool, user_scope: bool) {
        let path = self.at(".claude/fake-plugin-state.txt");
        let text = std::fs::read_to_string(&path).expect("Claude fake state exists");
        let mut fields: Vec<&str> = text.split('|').take(4).collect();
        assert_eq!(fields.len(), 4, "Claude fake state has its core fields");
        fields.push(if enabled { "true" } else { "false" });
        fields.push(if user_scope { "true" } else { "false" });
        std::fs::write(path, fields.join("|")).expect("Claude list witness mutates");
    }
}

pub(crate) fn target(client: PluginClient, id: &str, artifact: &str, name: &str) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key(&format!("deploy:{}-plugin", client.as_str())),
        provider: None,
        depends_on: None,
        config: Some(config(&format!("name = \"{name}\""))),
    }
}

pub(crate) fn selection(id: &str) -> DeploySelection {
    DeploySelection {
        profile: "local".to_owned(),
        targets: vec![id.to_owned()],
    }
}

pub(crate) fn write(root: &Path, relative: &str, bytes: &str) {
    let path = relative
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture parent");
    }
    std::fs::write(path, bytes).expect("fixture writes");
}

fn compile_clients(root: &Path) -> ClientExecutables {
    let source = root.join("fake-client.rs");
    std::fs::write(&source, FAKE_CLIENT).expect("fake source writes");
    let built = root.join(if cfg!(windows) {
        "fake-client.exe"
    } else {
        "fake-client"
    });
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let status = Command::new(rustc)
        .args(["--edition=2021", "-o"])
        .arg(&built)
        .arg(&source)
        .status()
        .expect("injected test rustc starts");
    assert!(status.success(), "fake client compiles");
    let install = |name: &str| {
        let path = root.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        });
        std::fs::copy(&built, &path).expect("fake client copies");
        ClientExecutable::Resolved {
            command: name.to_owned(),
            path,
        }
    };
    ClientExecutables {
        claude: install("claude"),
        codex: install("codex"),
        opencode: install("opencode"),
    }
}

const FAKE_CLIENT: &str = r###"
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let exe = env::current_exe().unwrap();
    let client = exe.file_stem().unwrap().to_string_lossy().to_lowercase();
    let client = if client.contains("opencode") { "opencode" } else if client.contains("codex") { "codex" } else { "claude" };
    let args: Vec<String> = env::args().skip(1).collect();
    let home = PathBuf::from(env::var_os("HOME").expect("HOME injected"));
    audit_env(client, &home);
    trace(&exe, client, &args);
    if args == ["--version"] {
        println!("{}", match client { "claude" => "Claude Code 2.1.9", "codex" => "codex-cli 0.148.3", _ => "opencode 1.17.4" });
        return;
    }
    if client == "opencode" { fail("OpenCode plugin command invoked"); }
    let root = private_root(client, &home);
    let state_path = root.join("fake-plugin-state.txt");
    let mut state = read_state(&state_path);
    match (client, args.as_slice()) {
        (_, [a,b,c,..]) if a == "plugin" && b == "marketplace" && c == "add" => {
            let marketplace_root = PathBuf::from(args.last().unwrap());
            let manifest = if client == "claude" { marketplace_root.join(".claude-plugin/marketplace.json") } else { marketplace_root.join("marketplace.json") };
            let text = fs::read_to_string(manifest).unwrap();
            state.marketplace = json_string(&text, "name");
            state.plugin = plugin_name(&text);
            let projected = marketplace_root.join("plugins").join(&state.plugin).join(if client == "claude" { ".claude-plugin/plugin.json" } else { ".codex-plugin/plugin.json" });
            state.version = json_string(&fs::read_to_string(projected).unwrap(), "version");
            write_state(&state_path, &state);
        }
        ("claude", [a,b,scope,user,coordinate]) if a == "plugin" && b == "install" && scope == "--scope" && user == "user" => install(&state_path, &mut state, coordinate),
        ("codex", [a,b,json,coordinate]) if a == "plugin" && b == "add" && json == "--json" => install(&state_path, &mut state, coordinate),
        ("claude", [a,b,json]) if a == "plugin" && b == "list" && json == "--json" => print_claude(&state),
        ("codex", [a,b,json]) if a == "plugin" && b == "list" && json == "--json" => print_codex(&state),
        ("claude", [a,b,scope,user,coordinate]) if a == "plugin" && b == "uninstall" && scope == "--scope" && user == "user" => remove(&state_path, &mut state, coordinate),
        ("codex", [a,b,json,coordinate]) if a == "plugin" && b == "remove" && json == "--json" => remove(&state_path, &mut state, coordinate),
        _ => fail(&format!("unexpected argv: {:?}", args)),
    }
}

#[derive(Default)] struct State { marketplace: String, plugin: String, version: String, installed: bool, enabled: bool, user_scope: bool }
fn private_root(client: &str, home: &Path) -> PathBuf {
    if client == "claude" { PathBuf::from(env::var_os("CLAUDE_CONFIG_DIR").unwrap()) }
    else { PathBuf::from(env::var_os("CODEX_HOME").unwrap_or_else(|| home.join(".codex").into_os_string())) }
}
fn read_state(path: &Path) -> State {
    let text = fs::read_to_string(path).unwrap_or_default(); let mut p = text.split('|');
    State { marketplace: p.next().unwrap_or("").into(), plugin: p.next().unwrap_or("").into(), version: p.next().unwrap_or("").into(), installed: p.next() == Some("true"), enabled: p.next().unwrap_or("true") == "true", user_scope: p.next().unwrap_or("true") == "true" }
}
fn write_state(path: &Path, s: &State) { fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(path, format!("{}|{}|{}|{}|{}|{}", s.marketplace,s.plugin,s.version,s.installed,s.enabled,s.user_scope)).unwrap(); }
fn install(path: &Path, s: &mut State, coordinate: &str) { if coordinate != format!("{}@{}",s.plugin,s.marketplace) { fail("wrong coordinate"); } s.installed=true; s.enabled=true; s.user_scope=true; write_state(path,s); }
fn remove(path: &Path, s: &mut State, coordinate: &str) { if coordinate != format!("{}@{}",s.plugin,s.marketplace) { fail("wrong coordinate"); } s.installed=false; write_state(path,s); }
fn print_claude(s: &State) { if s.installed { println!(r#"[{{"id":"{}@{}","version":"{}","scope":"{}","enabled":{}}}]"#,s.plugin,s.marketplace,s.version,if s.user_scope {"user"} else {"project"},s.enabled); } else { println!("[]"); } }
fn entry(s: &State, installed: bool) -> String { format!(r#"{{"pluginId":"{}@{}","name":"{}","marketplaceName":"{}","version":"{}","installed":{},"enabled":{}}}"#,s.plugin,s.marketplace,s.plugin,s.marketplace,s.version,installed,installed && s.enabled) }
fn print_codex(s: &State) { let i=if s.installed { entry(s,true) } else { String::new() }; println!(r#"{{"installed":[{}],"available":[{}]}}"#,i,entry(s,s.installed)); }
fn json_string(text: &str, key: &str) -> String { let marker=format!(r#""{}""#,key); let at=text.find(&marker).unwrap()+marker.len(); let tail=&text[at..]; let q=tail.find('"').unwrap(); let rest=&tail[q+1..]; rest[..rest.find('"').unwrap()].to_owned() }
fn plugin_name(text: &str) -> String { let at=text.find("\"plugins\"").unwrap(); json_string(&text[at..], "name") }
fn trace(exe: &Path, client: &str, args: &[String]) { let mut keys:Vec<String>=env::vars_os().map(|(k,_)|k.to_string_lossy().into_owned()).collect(); keys.sort(); let line=format!("{}|{}|{}\n",client,args.join("\t"),keys.join(",")); use std::io::Write; let mut f=fs::OpenOptions::new().create(true).append(true).open(exe.parent().unwrap().join("fake-client.trace")).unwrap(); f.write_all(line.as_bytes()).unwrap(); }
fn audit_env(client: &str, home: &Path) { let keys:BTreeSet<String>=env::vars_os().map(|(k,_)|k.to_string_lossy().into_owned()).collect(); let allowed:BTreeSet<String>=["SystemRoot","WINDIR","TEMP","TMP","LANG","LC_ALL","HOME","USERPROFILE",if client=="claude"{"CLAUDE_CONFIG_DIR"}else if client=="codex"{"CODEX_HOME"}else{"HOME"}].into_iter().map(str::to_owned).collect(); if !keys.is_subset(&allowed) { fail(&format!("leaked env: {:?}",keys.difference(&allowed).collect::<Vec<_>>())); } if env::var_os("USERPROFILE").as_deref()!=Some(home.as_os_str()) { fail("USERPROFILE mismatch"); } }
fn fail(message:&str)->! { eprintln!("{message}"); std::process::exit(91) }
"###;
