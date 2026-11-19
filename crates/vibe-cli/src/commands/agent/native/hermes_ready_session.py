"""Transport only: exact existing history on a pinned READY local owner.

No Vibe instruction composition, plugin registration, ensure/start or redial.
"""
import asyncio
import os
from pathlib import Path
import sys


def endpoint_identity(endpoint):
    return (endpoint.profile_id, endpoint.instance_id, endpoint.authority_epoch,
            endpoint.runtime_protocol, endpoint.api_origin, endpoint.control_home)


def discover(home):
    from hermes_cli.gateway_runtime import discover_gateway_endpoint
    found = discover_gateway_endpoint(home)
    if found.state != 'ready' or found.endpoint is None:
        raise ValueError('selected profile has no READY local owner')
    return found.endpoint


def snapshot_cwd(snapshot, sid):
    cwd = (snapshot.get('info') or {}).get('cwd')
    if snapshot.get('stored_session_id') != sid or not isinstance(cwd, str):
        raise ValueError('exact native session required')
    path = Path(cwd)
    if not path.is_absolute() or not path.is_dir():
        raise ValueError('native session workspace unavailable')
    return str(path.resolve())


def select_profile(args):
    # Use native profile APIs without main's interrupted-update recovery/relaunch.
    from hermes_cli._startup_fast import normalize_hermes_home_env
    normalize_hermes_home_env()
    from hermes_cli.profiles import resolve_profile_env, get_active_profile
    if args:
        if len(args) != 2 or args[0] != '--profile':
            raise ValueError('unsupported READY native arguments')
        os.environ['HERMES_HOME'] = resolve_profile_env(args[1])
    elif Path(os.environ.get('HERMES_HOME', '')).parent.name != 'profiles':
        active = get_active_profile()
        if active and active != 'default':
            os.environ['HERMES_HOME'] = resolve_profile_env(active)


async def run(sid, args):
    select_profile(args)
    if os.environ.get('HERMES_TUI_GATEWAY_URL'):
        raise ValueError('external gateway URL is not selected local ownership')
    from hermes_constants import get_hermes_home
    from hermes_cli.gateway_client import connect_gateway
    from hermes_cli.gateway_chat_view import GatewayChatView
    home = get_hermes_home()
    endpoint = discover(home)
    owner = endpoint_identity(endpoint)
    async with connect_gateway(endpoint=endpoint) as client:
        snapshot = await client.rpc('session.resume', session_id=sid)
        cwd = snapshot_cwd(snapshot, sid)

        async def recheck():
            if endpoint_identity(discover(home)) != owner:
                raise ValueError('selected READY owner changed; nothing retried')
            current = await client.rpc('session.resume', session_id=sid, omit_messages=True)
            if snapshot_cwd(current, sid) != cwd:
                raise ValueError('native session workspace changed')
            return current

        class BoundView(GatewayChatView):
            async def submit(self, text):
                if self.session_id != sid:
                    raise ValueError('another conversation requires explicit Vibe selection')
                await recheck()
                return await super().submit(text)

            async def command(self, text):
                command = text.partition(' ')[0]
                if command not in {'/quit', '/exit', '/detach', '/stop', '/approve', '/answer', '/discard', '/help'}:
                    print('Vibe READY transport: this command needs a separately admitted native route.', file=sys.stderr)
                    return True
                current = None
                if command not in {'/quit', '/exit', '/detach', '/help'}:
                    current = await recheck()
                if command == '/discard':
                    from hermes_cli.gateway_chat_view import lost_admission
                    lost = lost_admission(current, text.partition(' ')[2].strip())
                    await client.rpc('prompt.resolve_unknown', session_id=sid,
                        admission_id=lost['admission_id'], execution_generation=lost['execution_generation'])
                    return True
                return await super().command(text)

        await recheck()  # Retain the one full initial history for native display.
        print('Vibe Hermes: ready-session-transport; no instructions added; strict delivery unavailable.', file=sys.stderr)
        view = BoundView(client, snapshot)
        from agent.context_compressor import MODEL_ONLY_DISPLAY_METADATA_KEY
        for row in snapshot.get('messages', []):
            metadata = row.get('display_metadata')
            if isinstance(metadata, dict) and metadata.get(MODEL_ONLY_DISPLAY_METADATA_KEY):
                continue
            if row.get('role') in {'user', 'assistant'} and isinstance(row.get('content'), str):
                print(f"{row['role']}: {row['content']}")
        return await view.run()


def main():
    sid, *args = sys.argv[1:]
    try:
        return asyncio.run(run(sid, args))
    except (KeyboardInterrupt, asyncio.CancelledError):
        return 130
    except Exception:
        # Native exceptions may contain URLs, configuration or private history.
        print('Vibe Hermes READY transport refused or disconnected; no automatic retry. Check the selected owner and exact session. A sent turn may have an unknown outcome.', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
