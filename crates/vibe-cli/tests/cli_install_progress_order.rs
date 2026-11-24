//! Public install progress: live hierarchy, transient detail, and plain completion order.
#![cfg(unix)]

use std::process::Command;

// The driver owns a real 100x40 PTY and a separate pipe run. Each hook waits
// for the driver, so a snapshot cannot accidentally describe a completed step.
// Interpret the terminal, including cursor movement and erasure, rather than
// treating old bytes in the transcript as currently visible output.
const DRIVER: &str = r#"
import codecs, errno, json, os, pathlib, pty, select, struct, subprocess, sys, termios, time, fcntl
binary, base = sys.argv[1:]
base = pathlib.Path(base)

class Screen:
    def __init__(self):
        self.rows = [[' '] * 100 for _ in range(40)]
        self.r = self.c = 0
        self.escape = ''
        self.decoder = codecs.getincrementaldecoder('utf-8')('replace')
    def down(self):
        self.r += 1
        if self.r == 40:
            self.rows.pop(0)
            self.rows.append([' '] * 100)
            self.r = 39
    def feed(self, data):
        for ch in self.decoder.decode(data):
            if self.escape:
                self.escape += ch
                if self.escape == '\x1b[':
                    continue
                if self.escape.startswith('\x1b['):
                    if not ('@' <= ch <= '~'):
                        continue
                    raw = self.escape[2:-1]
                    args = [int(v or '0') for v in raw.lstrip('?').split(';')] if raw.lstrip('?') else [0]
                    n = args[0] or 1
                    if ch == 'A': self.r = max(0, self.r - n)
                    elif ch == 'B': self.r = min(39, self.r + n)
                    elif ch == 'C': self.c = min(99, self.c + n)
                    elif ch == 'D': self.c = max(0, self.c - n)
                    elif ch == 'G': self.c = min(99, n - 1)
                    elif ch in ('H', 'f'):
                        self.r = min(39, n - 1)
                        self.c = min(99, (args[1] or 1) - 1) if len(args) > 1 else 0
                    elif ch == 'K':
                        start, end = (0, 100) if args[0] == 2 else ((0, self.c + 1) if args[0] == 1 else (self.c, 100))
                        self.rows[self.r][start:end] = [' '] * (end - start)
                    elif ch == 'J':
                        if args[0] in (2, 3): self.rows = [[' '] * 100 for _ in range(40)]
                        elif args[0] == 0:
                            self.rows[self.r][self.c:] = [' '] * (100 - self.c)
                            for row in range(self.r + 1, 40): self.rows[row] = [' '] * 100
                        else: raise AssertionError('unsupported erase mode ' + raw)
                    elif ch not in ('m', 'h', 'l'):
                        raise AssertionError('unmodelled terminal command ' + repr(self.escape))
                    self.escape = ''
                elif len(self.escape) > 1:
                    raise AssertionError('unmodelled terminal escape ' + repr(self.escape))
                continue
            if ch == '\x1b': self.escape = ch
            elif ch == '\r': self.c = 0
            elif ch == '\n': self.down()
            elif ch == '\b': self.c = max(0, self.c - 1)
            elif ch == '\t': self.c = min(99, (self.c // 8 + 1) * 8)
            elif ch >= ' ':
                if self.c == 100:
                    self.c = 0
                    self.down()
                self.rows[self.r][self.c] = ch
                self.c += 1
    def text(self):
        return '\n'.join(''.join(row).rstrip() for row in self.rows)

reports = {}
for mode in ('tty', 'plain'):
    root = base / mode
    home, settings, project, registry = [root / name for name in ('home', 'settings', 'project', 'registry')]
    for directory in (home, settings, project, registry): directory.mkdir(parents=True)
    env = dict(os.environ)
    for key in list(env):
        if key.startswith('VIBE_') or key in ('NO_COLOR', 'CLICOLOR', 'CLICOLOR_FORCE', 'FORCE_COLOR'):
            env.pop(key)
    env.update(HOME=str(home), USERPROFILE=str(home), XDG_CONFIG_HOME=str(home / '.config'),
               XDG_DATA_HOME=str(home / '.local/share'), XDG_CACHE_HOME=str(home / '.cache'),
               VIBE_SETTINGS=str(settings), VIBE_NO_DEFAULT_REGISTRY='1', TERM='xterm-256color')
    init = subprocess.run([binary, 'init', '--path', str(project), '--group', 'org.test',
                           '--name', 'progress', '--kind', 'flow'], env=env, capture_output=True, timeout=30)
    assert init.returncode == 0, init.stdout + init.stderr
    package = registry / 'org.fixture/root/v0.1.0'
    (package / 'hooks').mkdir(parents=True)
    manifest = '[package]\ngroup="org.fixture"\nname="root"\nkind="tool"\nversion="0.1.0"\n'
    for name in ('one', 'two'):
        manifest += '\n[[extension]]\nid="order-' + name + '"\npoint="slot:post-install"\ntitle="Order ' + name + '"\nhandler={kind="script",base="hooks/' + name + '"}\napplies_to={packages=["org.fixture/root"]}\n'
        (package / ('hooks/' + name + '.sh')).write_text(
            'set -eu\nprintf "DETAIL-' + name + '\\n"\n' +
            'touch "$VIBE_PROJECT_ROOT/ready-' + name + '"\n' +
            'while [ ! -f "$VIBE_PROJECT_ROOT/release-' + name + '" ]; do sleep 0.05; done\n')
    (package / 'vibe.toml').write_text(manifest)
    args = [binary, '--verbose'] + (['--plain'] if mode == 'plain' else []) + [
        'install', 'org.fixture/root@^0.1', '--offline', '--assume-yes',
        '--registry', str(registry), '--path', str(project)]
    master = slave = None
    if mode == 'tty':
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 100, 0, 0))
        process = subprocess.Popen(args, env=env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        fd = master
    else:
        process = subprocess.Popen(args, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        fd = process.stdout.fileno()
    os.set_blocking(fd, False)
    raw = bytearray()
    screen = Screen()
    def drain():
        while select.select([fd], [], [], 0)[0]:
            try: data = os.read(fd, 65536)
            except OSError as error:
                if error.errno == errno.EIO: return
                raise
            if not data: return
            raw.extend(data)
            if mode == 'tty': screen.feed(data)
    snapshots = []
    try:
        for name in ('one', 'two'):
            deadline = time.monotonic() + 30
            while not (project / ('ready-' + name)).exists():
                drain()
                assert process.poll() is None, bytes(raw).decode(errors='replace')
                assert time.monotonic() < deadline, 'hook never became active: ' + bytes(raw).decode(errors='replace')
                time.sleep(0.01)
            # Let the bounded-rate renderer settle while the real child is blocked.
            until = time.monotonic() + 0.5
            while time.monotonic() < until:
                drain()
                time.sleep(0.01)
            assert process.poll() is None, 'live positive control exited early'
            snapshots.append(screen.text() if mode == 'tty' else bytes(raw).decode(errors='replace'))
            (project / ('release-' + name)).touch()
        deadline = time.monotonic() + 30
        while process.poll() is None:
            drain()
            assert time.monotonic() < deadline, 'install did not finish'
            time.sleep(0.01)
        drain()
        assert process.returncode == 0, bytes(raw).decode(errors='replace')
        reports[mode] = dict(live=snapshots, final=screen.text() if mode == 'tty' else bytes(raw).decode(errors='replace'))
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        if master is not None: os.close(master)
print(json.dumps(reports))
"#;

#[test]
fn plain_stream_matches_tty_block_order() {
    let scratch = tempfile::tempdir().unwrap();
    let output = Command::new("python3")
        .args(["-I", "-c", DRIVER, env!("CARGO_BIN_EXE_vibe")])
        .arg(scratch.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "driver failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let tty = &report["tty"];
    for (index, name) in ["one", "two"].iter().enumerate() {
        let live = tty["live"][index].as_str().unwrap();
        let lines: Vec<_> = live.lines().collect();
        let parent = lines.iter().position(|line| line.contains("Installing packages"));
        let child = lines.iter().position(|line| {
            line.contains(&format!("order-{name}")) && !line.contains("DETAIL-")
        });
        let detail = lines.iter().position(|line| line.contains(&format!("DETAIL-{name}")));
        assert!(
            matches!((parent, child, detail), (Some(p), Some(c), Some(d)) if p < c && c < d),
            "live parent must be above its active child, with detail underneath: {live}"
        );
        assert!(
            !lines[child.unwrap()].trim_start().starts_with('✓'),
            "the child must actually be running: {live}"
        );
        if index == 1 {
            assert!(!live.contains("DETAIL-one"), "finished detail must collapse: {live}");
        }
    }
    let final_screen = tty["final"].as_str().unwrap();
    assert!(!final_screen.contains("DETAIL-"), "detail survived completion: {final_screen}");
    assert!(final_screen.contains('✻'), "no successful trailer: {final_screen}");

    let plain = &report["plain"];
    let during_second = plain["live"][1].as_str().unwrap();
    let child = during_second.lines().find(|line| {
        line.starts_with("✓ ") && line.contains("order-one")
    }).unwrap_or_else(|| panic!("completed child was not flushed while parent ran: {during_second}"));
    assert!(child.contains("└ "), "plain child lost its indentation: {child}");
    assert!(
        !during_second.lines().any(|line| line.starts_with("✓ Installing packages")),
        "parent completed before the held second child: {during_second}"
    );
    let final_plain = plain["final"].as_str().unwrap();
    let first = final_plain.find(child).unwrap();
    let parent = final_plain.find("✓ Installing packages").unwrap();
    assert!(first < parent, "plain output changed temporal completion order: {final_plain}");
    assert!(final_plain.contains('✻'), "no successful plain trailer: {final_plain}");
    assert!(!final_plain.contains('\u{1b}'), "plain stream used cursor control: {final_plain}");
}
