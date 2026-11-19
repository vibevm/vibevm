"""Render one MCP key on a private captured YAML stage, never the live config.

The Rust registration owner retains publication and rollback authority. Requests
contain owned MCP payloads and private file references, never a complete config.
Owned values may themselves be sensitive; no request/result is logged or printed.
"""
import json
import os
from pathlib import Path
import stat
import sys


def run(request_path):
    request = json.loads(Path(request_path).read_text(encoding='utf-8'))
    if set(request) != {'stage', 'home', 'name', 'operation', 'value', 'result'}:
        raise ValueError('invalid native MCP render request')
    if os.environ.get('HERMES_CONFIG_BACKEND', 'file') != 'file':
        raise ValueError('native MCP rendering requires the admitted file backend')
    home = Path(request['home'])
    stage = Path(request['stage'])
    result = Path(request['result'])
    if not home.is_absolute() or not stage.is_absolute() or not result.is_absolute():
        raise ValueError('native MCP staging requires absolute paths')
    if '..' in home.parts or '..' in stage.parts or '..' in result.parts:
        raise ValueError('native MCP staging refuses parent traversal')
    stage.relative_to(home)
    result.relative_to(home)
    if stage == home / 'config.yaml' or result == home / 'config.yaml':
        raise ValueError('native renderer cannot write the live configuration')
    for path in (stage, result):
        if path.is_symlink() or not stat.S_ISREG(path.stat().st_mode):
            raise ValueError('native MCP staging requires owned regular files')
    from hermes_cli.config_backend import FileBackend, Changes
    document = FileBackend.read_path(stage)
    if document is None:
        document = {}
    if not isinstance(document, dict):
        raise ValueError('native configuration root is not a mapping')
    servers = document.get('mcp_servers', {})
    if not isinstance(servers, dict):
        raise ValueError('native MCP section is not a mapping')
    name = request['name']
    if not isinstance(name, str) or not name:
        raise ValueError('native MCP name is required')
    operation = request['operation']
    if operation == 'render':
        desired = request['value']
        if desired is None:
            servers.pop(name, None)
        else:
            from hermes_cli.mcp_security import validate_mcp_server_entry
            if validate_mcp_server_entry(name, desired):
                raise ValueError('native MCP entry validation refused')
            servers[name] = desired
        if servers:
            document['mcp_servers'] = servers
        else:
            document.pop('mcp_servers', None)
        # Native serializer only writes the invocation-owned captured stage.
        FileBackend.write_path(stage, Changes(document=document))
    elif operation != 'inspect':
        raise ValueError('unknown native MCP operation')
    observed = FileBackend.read_path(stage) or {}
    owned = observed.get('mcp_servers', {}).get(name)
    # Only the owned entry crosses this result boundary, not foreign config.
    with result.open('w', encoding='utf-8') as output:
        json.dump({'entry': owned}, output)
        output.write('\n')


if __name__ == '__main__':
    try:
        if len(sys.argv) == 3 and sys.argv[1] == '--profile-home':
            output = Path(sys.argv[2])
            sys.argv = ['hermes', 'chat']
            from hermes_cli.main import _apply_profile_override
            _apply_profile_override()
            from hermes_constants import get_hermes_home
            with output.open('w', encoding='utf-8') as stream:
                json.dump({'home': str(get_hermes_home())}, stream)
        else:
            run(sys.argv[1])
    except Exception as error:
        # Never echo native parser/config exceptions that may contain secrets.
        print('Hermes MCP staging refused: validate runtime, profile and owned staging files', file=sys.stderr)
        sys.exit(1)
