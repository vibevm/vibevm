//! Real offline self apply on one isolated 100-column terminal.
#![cfg(target_os = "linux")]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#trailer");

use std::fs;
use std::process::Command;

// One shared terminal, including a small screen model: transient progress redraws
// are not completed rows. Keep scrollback as well as the final 40-line viewport.
const PTY: &str = r#"
import errno, fcntl, json, os, pty, re, select, signal, struct, subprocess, sys, termios, time, tty, unicodedata
master, slave = pty.openpty()
tty.setraw(slave)
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 100, 0, 0))
child = None
raw = bytearray()
deadline = time.monotonic() + 45
try:
    child = subprocess.Popen(sys.argv[1:], stdin=slave, stdout=slave, stderr=slave,
                             env=dict(os.environ), start_new_session=True)
    os.close(slave)
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError('self apply PTY deadline exceeded')
        ready, _, _ = select.select([master], [], [], remaining)
        if not ready:
            raise RuntimeError('self apply PTY deadline exceeded')
        try:
            data = os.read(master, 65536)
        except OSError as error:
            if error.errno != errno.EIO:
                raise
            data = b''
        if not data:
            break
        raw.extend(data)
        if len(raw) > 1048576:
            raise RuntimeError('self apply PTY output exceeded limit')
    status = child.wait(timeout=max(0.01, deadline - time.monotonic()))
finally:
    if child is not None and child.poll() is None:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait(timeout=2)
    for fd in [master, slave]:
        try:
            os.close(fd)
        except OSError:
            pass
screen = [[' '] * 100 for _ in range(40)]
history = []
y = x = 0
saved = (0, 0)
def down():
    global y
    y += 1
    if y == 40:
        history.append(''.join(screen.pop(0)).rstrip())
        screen.append([' '] * 100)
        y = 39
text = raw.decode('utf-8')
i = 0
while i < len(text):
    c = text[i]
    if c == '\x1b':
        match = re.match(r'\x1b\[([0-9;?]*)([ -/]*)([@-~])', text[i:])
        if not match:
            raise RuntimeError('unsupported terminal escape')
        params, _, op = match.groups()
        nums = [int(n or '0') for n in params.lstrip('?').split(';')]
        n = nums[0] or 1
        if op == 'A': y = max(0, y - n)
        elif op == 'B': y = min(39, y + n)
        elif op == 'C': x = min(99, x + n)
        elif op == 'D': x = max(0, x - n)
        elif op == 'G': x = min(99, n - 1)
        elif op in ['H', 'f']:
            y = min(39, n - 1)
            x = min(99, (nums[1] or 1) - 1) if len(nums) > 1 else 0
        elif op == 'K':
            mode = nums[0]
            start, end = (0, 100) if mode == 2 else ((0, x + 1) if mode == 1 else (x, 100))
            screen[y][start:end] = [' '] * (end - start)
        elif op == 'J':
            if nums[0] in [2, 3]: screen = [[' '] * 100 for _ in range(40)]
            elif nums[0] == 0:
                screen[y][x:] = [' '] * (100 - x)
                for row in range(y + 1, 40): screen[row] = [' '] * 100
            else: raise RuntimeError('unsupported erase mode')
        elif op == 's': saved = (y, x)
        elif op == 'u': y, x = saved
        elif op not in ['m', 'h', 'l']: raise RuntimeError('unsupported terminal operation ' + op)
        i += len(match.group(0))
        continue
    if c == '\r': x = 0
    elif c == '\n':
        down()
        x = 0
    elif c == '\b': x = max(0, x - 1)
    elif c >= ' ':
        width = 0 if unicodedata.combining(c) else (2 if unicodedata.east_asian_width(c) in ['W', 'F'] else 1)
        if width:
            if x + width > 100:
                down()
                x = 0
            screen[y][x] = c
            if width == 2: screen[y][x + 1] = ''
            x += width
    i += 1
print(json.dumps([status, '\n'.join(history + [''.join(row).rstrip() for row in screen]).rstrip()]))
"#;

#[test]
fn self_apply_live_rows_in_order_and_one_receipt() {
    assert!(vibe_test_support::isolated_home().is_some());
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let settings = home.join("settings");
    let store = root.path().join("install");
    let loose = root.path().join("loose");
    for path in [&home, &settings, &store, &loose] {
        fs::create_dir_all(path).unwrap();
    }
    let binary = loose.join("vibe");
    fs::copy(env!("CARGO_BIN_EXE_vibe"), &binary).unwrap();
    assert_eq!(fs::read_dir(&loose).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&store).unwrap().count(), 0);
    let profile = root.path().join("corp.toml");
    fs::write(&profile, format!(
        "schema = 1\nname = \"corp\"\n[artifacts]\npackages = \"{}\"\n",
        root.path().join("payloads").display(),
    )).unwrap();
    let mut command = Command::new("/usr/bin/python3");
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("VIBE_SETTINGS", &settings)
        .env("VIBEVM_INSTALL_ROOT", &store)
        .env("VIBE_REGISTRY_CACHE", root.path().join("registry-cache"))
        .env("VIBEVM_SEARCH_CACHE_DIR", root.path().join("search-cache"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("SHELL", "/bin/bash")
        .env("TERM", "xterm-256color")
        .env("COLUMNS", "100")
        .env("LINES", "40")
        .current_dir(root.path())
        .args(["-I", "-c", PTY])
        .arg(&binary)
        .args(["self", "apply"])
        .arg(&profile)
        .args(["--no-fix-path", "--offline"]);
    let result = command.output().expect("isolated bounded PTY child");
    assert!(result.status.success(), "PTY setup failed: {:?}", String::from_utf8_lossy(&result.stderr));
    let (status, screen): (i32, String) = serde_json::from_slice(&result.stdout).unwrap();
    let excerpt = screen.chars().take(6000).collect::<String>();
    assert_eq!(status, 0, "self apply failed: {excerpt:?}");
    assert!(screen.contains("effective settings"), "retained settings: {excerpt:?}");
    assert!(screen.contains("where releases and payloads go"), "retained destinations: {excerpt:?}");
    assert!(screen.contains("PATH") && screen.contains("--fix-path"), "retained PATH and hint: {excerpt:?}");
    let rows = screen.lines().filter(|line| {
        (line.starts_with("✓ ") || line.starts_with("○ "))
            && line.split_whitespace().nth(1) != Some("PATH")
    }).collect::<Vec<_>>();
    let names = rows.iter().map(|line| line.split_whitespace().nth(1).unwrap()).collect::<Vec<_>>();
    assert_eq!(names, ["verify", "px", "copy", "record", "profile"], "completed operations: {excerpt:?}");
    for (index, title) in [(0, "local import payload"), (2, "import payload")] {
        let row = rows[index];
        assert!(row.starts_with("✓ ") && row.contains(title), "wrong completed row {row:?}");
        assert!(valid_elapsed(row.rsplit_once(" · ").unwrap().1));
    }
    let pin = vibe_net::px_store::PxPin::embedded();
    let asset = pin.asset(vibe_net::px_store::running_target().unwrap());
    let asset = asset.expect("fixture platform has a pinned px asset");
    let reason = format!("skipped: no {} beside the binary", pin.release_asset_name(asset));
    assert!(rows[1].starts_with("○ ") && rows[1].ends_with(&reason), "bounded skip reason: {:?}", rows[1]);
    let inventory: toml::Value = toml::from_str(&fs::read_to_string(store.join("opt/vibevm/state.toml")).unwrap()).unwrap();
    let installs = inventory["install"].as_array().unwrap();
    assert_eq!(installs.len(), 1);
    let installed = &installs[0];
    let selector = format!("{}:{}#{}", installed["kind"].as_str().unwrap(), installed["id"].as_str().unwrap(), installed["instance"].as_integer().unwrap());
    assert!(rows[3].starts_with("✓ ") && rows[3].contains(&format!("imported instance {selector}")));
    assert!(valid_elapsed(rows[3].rsplit_once(" · ").unwrap().1));
    assert!(rows[4].starts_with("✓ ") && rows[4].contains("corp written to ") && rows[4].contains("site-profile.toml"));
    assert_eq!(fs::read_to_string(settings.join("site-profile.toml")).unwrap(), fs::read_to_string(profile).unwrap());
    for old in ["Verifying local import payload", "Installing px", "Copying import payload", "Recording imported instance", "the running vibe as"] {
        assert!(!screen.contains(old), "old duplicate {old:?}: {excerpt:?}");
    }
    let trailer_at = screen.find("✻ ").expect("final receipt trailer");
    assert_eq!(screen.matches("✻ ").count(), 1, "one trailer: {excerpt:?}");
    let trailer = screen[trailer_at..].lines().collect::<String>();
    let prefix = "✻ applied site profile corp · 5 steps · ";
    let receipt = settings.join("site-profile.receipt.toml");
    let suffix = format!(" · {}", receipt.display());
    let elapsed = trailer.strip_prefix(prefix).and_then(|s| s.strip_suffix(&suffix)).unwrap_or_else(|| panic!("last trailer fields: {trailer:?}"));
    assert!(valid_elapsed(elapsed));
    assert!(receipt.is_file());
    // The same real invocation without colour must retain content and must
    // observe its own unchanged profile, not aggregate receipt/settings writes.
    command.env("NO_COLOR", "1");
    let repeated = command.output().expect("isolated repeated apply");
    assert!(repeated.status.success());
    let (status, repeated): (i32, String) = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(status, 0);
    assert!(repeated.contains("profile unchanged"), "repeat: {repeated:?}");
    assert!(!repeated.contains("written to"), "false profile write: {repeated:?}");
    assert!(repeated.contains("effective settings"));
    assert!(repeated.contains("where releases and payloads go"));
    assert!(repeated.contains("PATH") && repeated.contains("--fix-path"));
    assert_eq!(repeated.matches("✻ ").count(), 1);
    let trailer = repeated[repeated.find("✻ ").unwrap()..].lines().collect::<String>();
    assert!(trailer.starts_with("✻ site profile corp already applied · nothing changed · 1 step · "));
    assert!(trailer.ends_with(&suffix));
    let elapsed = trailer.strip_suffix(&suffix).unwrap().rsplit_once(" · ").unwrap().1;
    assert!(valid_elapsed(elapsed));
    assert!(receipt.is_file());
    assert!(!home.join(".bashrc").exists());
    assert!(!home.join(".profile").exists());
}

fn valid_elapsed(text: &str) -> bool {
    let number = |part: &str, unit: char| part.strip_suffix(unit).and_then(|n| n.parse::<u64>().ok());
    match text.split(' ').collect::<Vec<_>>().as_slice() {
        [one] => number(one, 's').is_some_and(|n| n < 60)
            || number(one, 'm').is_some_and(|n| (1..60).contains(&n))
            || number(one, 'h').is_some_and(|n| n > 0),
        [first, second] => (number(first, 'm').is_some_and(|n| (1..60).contains(&n)) && number(second, 's').is_some_and(|n| (1..60).contains(&n)))
            || (number(first, 'h').is_some_and(|n| n > 0) && number(second, 'm').is_some_and(|n| (1..60).contains(&n))),
        _ => false,
    }
}
