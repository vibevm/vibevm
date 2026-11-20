"""Public CLI acceptance probe. Run inside a network-isolated sandbox with /vibe."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def run(root, args, data=None, success=True):
    p = subprocess.run(['/vibe', *args, '--json'], input=data, capture_output=True,
                       timeout=15, env={'HOME': str(root), 'VIBE_SETTINGS': str(root),
                                       'PATH': '/usr/bin:/bin'})
    assert all(marker not in p.stdout + p.stderr for marker in VALUES), 'value in CLI output'
    assert (p.returncode == 0) == success, (args[0:3], p.returncode)
    return json.loads(p.stdout) if success else None


VALUES = [b'SYNTHETIC_FIRST_21e0', b'SYNTHETIC_SECOND_21e0']
with tempfile.TemporaryDirectory(dir='/tmp') as scratch:
    root = Path(scratch)
    neighbor = root / 'unrelated'
    neighbor.write_bytes(b'keep')
    call = lambda args, **kw: run(root, args, **kw)
    meta = call(['secrets', 'create', '--kind', 'api-key', '--label', 'Fixture', '--value-stdin'], data=VALUES[0])
    key = meta['items'][0]['id']
    rev = meta['items'][0]['current_revision']
    ai = ['ai', 'connections']
    call(ai + ['add', '--label', 'Fixture connection', '--endpoint', 'http://127.0.0.1:9817/v1', '--api-kind', 'openai-chat-completions', '--default-model', 'fixture-model', '--secret-ref', key])
    index = root / 'secrets/ai/index.toml'
    before = index.read_bytes()
    assert key.encode() in before
    listed = call(ai + ['list'])
    assert len(listed['items']) == 1
    connection, row = next(iter(listed['items'].items()))
    detail = call(ai + ['details', connection])
    assert row['secret_ref'] == key and detail['items'][connection] == row
    call(['secrets', 'rotate', key, '--expect-current', rev, '--value-stdin'], data=VALUES[1])
    assert index.read_bytes() == before and call(ai + ['details', connection]) == detail
    def remove_key(success):
        current = call(['secrets', 'details', key]); item = current['items'][0]
        call(['secrets', 'remove', key, '--expect-store', str(current['store_revision']), '--expect-current', item['current_revision'], '--yes'], success=success)
    key_index = root / 'secrets/index.toml'; selected = key_index.read_bytes()
    remove_key(False)
    assert key_index.read_bytes() == selected and index.read_bytes() == before
    token = detail['store_revision']
    call(ai + ['remove', connection, '--expect-store', str(token), '--expect-current', row['current_revision'], '--yes'])
    assert call(['secrets', 'details', key])['items'][0]['id'] == key
    remove_key(True)
    assert call(['secrets', 'list'])['items'] == [] and neighbor.read_bytes() == b'keep'
    print('PASS: rotation, referenced refusal, explicit unbind, final logical removal;', hashlib.file_digest(open('/vibe', 'rb'), 'sha256').hexdigest())
