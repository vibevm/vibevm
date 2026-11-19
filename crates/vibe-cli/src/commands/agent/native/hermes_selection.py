"""Native session selection before the Rust project/prefix decision.

The supervisor owns one profile reservation/runner across the private rendezvous.
Native clients retain their own history, validation, input and model loop.
"""
import asyncio
import argparse
import json
import os
from pathlib import Path
import stat
import sys
import time
import uuid


async def settle_owned(task):
    """Keep the exact owned task/result through repeated cancellation signals."""
    while True:
        try:
            return await asyncio.shield(task)
        except asyncio.CancelledError:
            if task.done():
                return task.result()


async def finish_owned(tree, session_id):
    from hermes_cli.session_worktree import finish
    task = asyncio.create_task(asyncio.to_thread(finish, tree, session_id, ask=False))
    await settle_owned(task)


def read_private(path):
    descriptor = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0))
    with os.fdopen(descriptor, 'rb') as source:
        if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
            raise ValueError('selection transport is not regular')
        data = source.read(65537)
    if len(data) > 65536:
        raise ValueError('selection transport exceeds bound')
    return json.loads(data)


def publish_private(path, value):
    encoded = json.dumps(value).encode('utf-8')
    descriptor = os.open(path, os.O_WRONLY | os.O_TRUNC | getattr(os, 'O_NOFOLLOW', 0))
    with os.fdopen(descriptor, 'wb') as output:
        if not stat.S_ISREG(os.fstat(output.fileno()).st_mode):
            raise ValueError('selection transport is not regular')
        output.write(encoded)
        output.flush()


async def select(client, parsed):
    from hermes_cli import gateway_chat as native
    from hermes_cli.gateway_client import GatewayClientError
    from hermes_cli.main_resume_args import apply_in_dir
    from hermes_cli.main_tui_launch import _resolve_use_tui, _bypass_chat_launch
    from hermes_cli.config import load_config_readonly
    # Freeze only the native classic finite-query worktree preference here.
    # The chosen gateway frontend never re-enters classic config-driven setup.
    finite_query = bool(getattr(parsed, 'query', None) or getattr(parsed, 'query_file', None))
    finite = bool(getattr(parsed, 'quiet', False) or getattr(parsed, 'oneshot_exit', False)
        or not (sys.stdin.isatty() and sys.stdout.isatty()))
    classic = not (_resolve_use_tui(parsed) or _bypass_chat_launch(parsed)
        or getattr(parsed, 'output_format', 'text') == 'stream-json')
    if finite_query and not (finite and classic):
        raise ValueError('selected native frontend requires its own admitted bootstrap; query routing refused')
    validation = argparse.Namespace(**vars(parsed))
    validation.worktree = False  # The existing external worktree owner consumes this effect below.
    validation.no_restore_cwd = False if getattr(parsed, 'in_dir', None) else getattr(parsed, 'no_restore_cwd', False)
    native.validate_options(validation)  # Refuse unsupported classic flags BEFORE any worktree effect.
    if finite_query and finite and classic:
        parsed.worktree = bool(getattr(parsed, 'worktree', False) or load_config_readonly().get('worktree', False))
    explicit_no_restore = getattr(parsed, 'no_restore_cwd', False)
    apply_in_dir(parsed)
    tree = None
    if getattr(parsed, 'worktree', False):
        from hermes_cli.session_worktree import create
        creation = asyncio.create_task(asyncio.to_thread(create))
        try:
            tree = await asyncio.shield(creation)
        except asyncio.CancelledError:
            # Native creation may still own a git subprocess: settle before cleanup.
            tree = await settle_owned(creation)
            if tree is not None:
                await finish_owned(tree, None)
            raise
        if tree is None:
            raise ValueError('native worktree creation failed')
        parsed.worktree = False  # Only this invocation; effect already consumed.
        parsed.in_dir = tree['path']
    try:
        validation = argparse.Namespace(**vars(parsed))
        validation.no_restore_cwd = explicit_no_restore  # apply_in_dir consumed the cwd effect.
        native.validate_options(validation)
        title = native.continue_title(parsed)
        if native.wants_latest(parsed):
            snapshot = await native._resume_latest(client, parsed)
        elif getattr(parsed, 'resume', None) or (title and not getattr(parsed, 'create_if_missing', False)):
            name = getattr(parsed, 'resume', None) or title
            try:
                snapshot = await native._resume(client, parsed, session_id=name)
            except GatewayClientError as error:
                if str(error) != 'not_found':
                    raise
                snapshot = await native._resume(client, parsed, title=name)
        else:
            from hermes_cli.gateway_session_store import _creation_policy
            source = getattr(parsed, 'source', None) or ('oneshot' if getattr(parsed, 'oneshot', None) else 'cli')
            policy = _creation_policy(vars(parsed))
            if title and getattr(parsed, 'create_if_missing', False):
                policy['title'] = title
            created = await client.rpc('session.create', request_id=uuid.uuid4().hex,
                source=source, cwd=native._caller_cwd(parsed), **policy)
            snapshot = await native._resume(client, parsed, session_id=created['session_id'])
        native.check_resume_policy(parsed, snapshot)
        session_id = snapshot['stored_session_id']
        from hermes_cli.session_worktree import restore
        restoration = asyncio.create_task(asyncio.to_thread(restore, session_id))
        try:
            await asyncio.shield(restoration)
        except asyncio.CancelledError:
            # Restore owns native retention metadata; settle it, never infer a new tree to delete.
            await settle_owned(restoration)
            raise
        cwd = (snapshot.get('info') or {}).get('cwd')
        if not isinstance(cwd, str) or not Path(cwd).is_absolute() or not Path(cwd).is_dir():
            raise ValueError('selected native session workspace unavailable')
        return session_id, str(Path(cwd).resolve()), tree
    except BaseException:
        if tree is not None:
            await finish_owned(tree, None)
        raise


async def prefix_ack(directory, session_id, cwd):
    directory = Path(directory)
    publish_private(directory / 'ready.json', {'session': session_id, 'cwd': cwd, 'mode': 'selected'})
    deadline = time.monotonic() + 120
    parent = os.getppid()
    if parent == 1:
        raise ValueError('selection parent exited before readiness')
    while time.monotonic() < deadline:
        if os.getppid() != parent:
            raise ValueError('selection parent exited')
        try:
            result = read_private(directory / 'ack.json')
        except json.JSONDecodeError:
            await asyncio.sleep(0.02)
            continue
        if set(result) != {'session', 'cwd', 'prefix'} or result['session'] != session_id or result['cwd'] != cwd:
            raise ValueError('selection acknowledgement binding changed')
        if result['prefix'] is None:
            return None
        path = Path(result['prefix'])
        if not path.is_absolute():
            raise ValueError('selected prefix path is not absolute')
        descriptor = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0))
        with os.fdopen(descriptor, 'rb') as source:
            if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                raise ValueError('selected prefix is not regular')
            prefix = source.read(16 * 1024 * 1024 + 1)
        if len(prefix) > 16 * 1024 * 1024:
            raise ValueError('selected prefix exceeds bound')
        return prefix.decode('utf-8', errors='strict')
    raise TimeoutError('selection prefix acknowledgement timed out')


def frontend(session_id, cwd, args):
    from hermes_cli.main import _light_chat_parser, _set_chat_arg_defaults
    parsed = _light_chat_parser().parse_args(args)
    _set_chat_arg_defaults(parsed)
    parsed.resume = session_id
    parsed.continue_last = None
    parsed.create_if_missing = False
    parsed.worktree = False
    parsed.in_dir = cwd
    parsed.no_restore_cwd = False
    from hermes_cli.gateway_chat_startup import launch_gateway_chat
    return launch_gateway_chat(parsed)
