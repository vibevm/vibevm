//! A loopback HTTP/1.1 server and a GitVerse release-API emulation of the
//! documented endpoints: release by tag / list / create, asset list /
//! multipart upload / delete, and the asset download URL.

use super::*;

#[derive(Debug, Clone)]
pub(super) struct Recorded {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) query: String,
    pub(super) authorization: Option<String>,
    pub(super) accept: Option<String>,
    pub(super) content_type: Option<String>,
    pub(super) body: Vec<u8>,
}

pub(super) type Reply = (u16, Vec<u8>);
pub(super) type Handler = Arc<dyn Fn(&Recorded, &str) -> Reply + Send + Sync>;

pub(super) struct FakeServer {
    pub(super) base: String,
    pub(super) log: Arc<Mutex<Vec<Recorded>>>,
}

impl FakeServer {
    pub(super) fn start(handler: Handler) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let log = Arc::new(Mutex::new(Vec::new()));
        let thread_log = log.clone();
        let thread_base = base.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                serve(stream, &handler, &thread_log, &thread_base);
            }
        });
        Self { base, log }
    }

    pub(super) fn requests(&self) -> Vec<Recorded> {
        self.log.lock().unwrap().clone()
    }

    pub(super) fn clear(&self) {
        self.log.lock().unwrap().clear();
    }
}

pub(super) fn serve(stream: TcpStream, handler: &Handler, log: &Mutex<Vec<Recorded>>, base: &str) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    let (path, query) = target
        .split_once('?')
        .map(|(p, q)| (p.to_string(), q.to_string()))
        .unwrap_or((target.clone(), String::new()));
    let mut headers = BTreeMap::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    let recorded = Recorded {
        method,
        path,
        query,
        authorization: headers.get("authorization").cloned(),
        accept: headers.get("accept").cloned(),
        content_type: headers.get("content-type").cloned(),
        body,
    };
    let (status, reply) = handler(&recorded, base);
    log.lock().unwrap().push(recorded);
    let mut stream = stream;
    let head = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n",
        reply.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&reply);
}

pub(super) fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

pub(super) fn json(value: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&value).unwrap()
}

#[derive(Default)]
pub(super) struct GvRelease {
    pub(super) id: u64,
    pub(super) tag: String,
    pub(super) draft: bool,
    pub(super) assets: Vec<(u64, String, Vec<u8>)>,
}

#[derive(Default)]
pub(super) struct GvState {
    pub(super) releases: Vec<GvRelease>,
    pub(super) next_id: u64,
    /// Emulates a host whose by-tag route does not show drafts.
    pub(super) by_tag_hides_drafts: bool,
}

pub(super) fn release_json(release: &GvRelease) -> serde_json::Value {
    serde_json::json!({
        "id": release.id, "tag_name": release.tag, "target_commitish": "master",
        "name": release.tag, "body": "", "draft": release.draft, "prerelease": false,
        "is_authorized_only": false, "assets": [],
    })
}

pub(super) fn asset_json(
    base: &str,
    release_id: u64,
    asset: &(u64, String, Vec<u8>),
) -> serde_json::Value {
    serde_json::json!({
        "id": asset.0, "name": asset.1, "size": asset.2.len(), "download_count": 0,
        "browser_download_url":
            format!("{base}{REPO}/releases/{release_id}/assets/{}/download", asset.0),
        "url": format!("{base}/public/api/attachments/u{}", asset.0),
        "created_at": "2026-11-08T00:00:00Z", "uuid": format!("u{}", asset.0),
    })
}

pub(super) fn not_found() -> Reply {
    (
        404,
        json(
            serde_json::json!({"error": {"code": "NOT_FOUND", "message": "not found", "status": 404}}),
        ),
    )
}

pub(super) fn parse_multipart(request: &Recorded) -> Option<Vec<u8>> {
    let boundary = request
        .content_type
        .as_deref()?
        .strip_prefix("multipart/form-data; boundary=")?
        .to_string();
    let body = &request.body;
    let start = format!("--{boundary}\r\n");
    let end = format!("\r\n--{boundary}--\r\n");
    if !body.starts_with(start.as_bytes()) || !body.ends_with(end.as_bytes()) {
        return None;
    }
    let inner = &body[start.len()..body.len() - end.len()];
    let split = inner.windows(4).position(|w| w == b"\r\n\r\n")?;
    let headers = std::str::from_utf8(&inner[..split]).ok()?;
    headers
        .contains("name=\"attachment\"")
        .then(|| inner[split + 4..].to_vec())
}

pub(super) fn gitverse_handler(state: Arc<Mutex<GvState>>) -> Handler {
    Arc::new(move |request, base| {
        if request.authorization.as_deref() != Some(&format!("Bearer {TOKEN}")) {
            return (
                401,
                json(serde_json::json!({"error": {"code": "UNAUTHORIZED"}})),
            );
        }
        let mut state = state.lock().unwrap();
        let Some(rest) = request.path.strip_prefix(REPO) else {
            return not_found();
        };
        let segments = rest.trim_start_matches('/').split('/').collect::<Vec<_>>();
        let download = matches!(
            segments.as_slice(),
            ["releases", _, "assets", _, "download"]
        );
        if !download && request.accept.as_deref() != Some(GITVERSE_ACCEPT) {
            return (400, Vec::new());
        }
        match (request.method.as_str(), segments.as_slice()) {
            ("GET", ["releases", "tags", tag]) => {
                let hide = state.by_tag_hides_drafts;
                match state
                    .releases
                    .iter()
                    .find(|r| r.tag == *tag && !(hide && r.draft))
                {
                    Some(release) => (200, json(release_json(release))),
                    None => not_found(),
                }
            }
            ("GET", ["releases"]) => {
                let page: usize = query_value(&request.query, "page")
                    .unwrap()
                    .parse()
                    .unwrap();
                let per: usize = query_value(&request.query, "per_page")
                    .unwrap()
                    .parse()
                    .unwrap();
                let items = state
                    .releases
                    .iter()
                    .skip((page - 1) * per)
                    .take(per)
                    .map(release_json)
                    .collect::<Vec<_>>();
                (200, json(serde_json::Value::Array(items)))
            }
            ("POST", ["releases"]) => {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let tag = body["tag_name"].as_str().unwrap().to_string();
                if state.releases.iter().any(|r| r.tag == tag) {
                    return (
                        409,
                        json(serde_json::json!({"error": {"code": "CONFLICT"}})),
                    );
                }
                state.next_id += 1;
                let release = GvRelease {
                    id: state.next_id,
                    tag,
                    draft: body["draft"].as_bool().unwrap(),
                    assets: Vec::new(),
                };
                let reply = json(release_json(&release));
                state.releases.push(release);
                (201, reply)
            }
            ("GET", ["releases", id, "assets"]) => {
                let page: usize = query_value(&request.query, "page")
                    .unwrap()
                    .parse()
                    .unwrap();
                let per: usize = query_value(&request.query, "per_page")
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!(per <= 50, "documented per_page maximum");
                let Some(release) = state.releases.iter().find(|r| r.id.to_string() == *id) else {
                    return not_found();
                };
                let items = release
                    .assets
                    .iter()
                    .skip((page - 1) * per)
                    .take(per)
                    .map(|asset| asset_json(base, release.id, asset))
                    .collect::<Vec<_>>();
                (200, json(serde_json::Value::Array(items)))
            }
            ("POST", ["releases", id, "assets"]) => {
                let Some(name) = query_value(&request.query, "name") else {
                    return (400, Vec::new());
                };
                let Some(bytes) = parse_multipart(request) else {
                    return (400, Vec::new());
                };
                state.next_id += 1;
                let asset_id = state.next_id;
                let Some(release) = state.releases.iter_mut().find(|r| r.id.to_string() == *id)
                else {
                    return not_found();
                };
                if release.assets.iter().any(|asset| asset.1 == name) {
                    return (
                        409,
                        json(serde_json::json!({"error": {"code": "DUPLICATE"}})),
                    );
                }
                release.assets.push((asset_id, name, bytes));
                let reply = json(asset_json(base, release.id, release.assets.last().unwrap()));
                (201, reply)
            }
            ("DELETE", ["releases", id, "assets", asset_id]) => {
                let Some(release) = state.releases.iter_mut().find(|r| r.id.to_string() == *id)
                else {
                    return not_found();
                };
                let before = release.assets.len();
                release
                    .assets
                    .retain(|asset| asset.0.to_string() != *asset_id);
                if release.assets.len() == before {
                    return not_found();
                }
                (204, Vec::new())
            }
            ("GET", ["releases", id, "assets", asset_id, "download"]) => state
                .releases
                .iter()
                .find(|r| r.id.to_string() == *id)
                .and_then(|r| r.assets.iter().find(|a| a.0.to_string() == *asset_id))
                .map(|asset| (200, asset.2.clone()))
                .unwrap_or_else(not_found),
            _ => not_found(),
        }
    })
}

pub(super) fn gitverse_fake() -> (FakeServer, Arc<Mutex<GvState>>) {
    let state = Arc::new(Mutex::new(GvState {
        next_id: 100,
        ..GvState::default()
    }));
    (FakeServer::start(gitverse_handler(state.clone())), state)
}

pub(super) fn client_for(server: &FakeServer) -> GitverseClient {
    GitverseClient::new(
        Endpoint::new(&server.base, "vibevm", "vibevm").unwrap(),
        Token::from_explicit(TOKEN),
    )
    .unwrap()
}

pub(super) fn mirror(
    server: &FakeServer,
    source: &SourceRelease,
    replace: bool,
) -> Result<(MirrorReport, String)> {
    let mut out = Vec::new();
    let report = mirror_with(&client_for(server), source, replace, &mut out)?;
    let out = String::from_utf8(out).unwrap();
    assert!(!out.contains(TOKEN));
    Ok((report, out))
}

pub(super) fn uploads(requests: &[Recorded]) -> Vec<String> {
    requests
        .iter()
        .filter(|r| r.method == "POST" && r.path.ends_with("/assets"))
        .map(|r| query_value(&r.query, "name").unwrap())
        .collect()
}
