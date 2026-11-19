"""Native Hermes bootstrap supervisor; not enabled by Rust launch admission yet."""
import asyncio
import importlib.util
import os
from pathlib import Path
import signal
import sys
import uuid


async def run_native_hermes(carrier_path, prefix_path, cwd, args):
    os.chdir(cwd)  # Isolated helper process, not the host CLI's global cwd.
    # Native parser owns syntax; selectors not yet mapped must refuse before effects.
    from hermes_cli.main import _light_chat_parser
    parsed = _light_chat_parser().parse_args(args)
    if getattr(parsed, 'command', None) not in (None, 'chat'):
        raise RuntimeError('Vibe Hermes bootstrap admits native chat only at this stage')
    if any(getattr(parsed, key, None) for key in ('resume', 'continue_last', 'worktree', 'oneshot')):
        raise RuntimeError('Vibe Hermes session selector requires native re-admission; use read delivery until admitted')
    if os.environ.get('HERMES_TUI_GATEWAY_URL'):
        raise RuntimeError('Vibe cannot attach an owned carrier to an external Hermes runner; use read delivery')
    prefix = Path(prefix_path).read_bytes().decode('utf-8', errors='strict')
    if not prefix:
        raise RuntimeError('Vibe Hermes prefix is empty')
    spec = importlib.util.spec_from_file_location('vibe_owned_hermes_carrier', carrier_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    from hermes_constants import get_hermes_home
    from gateway.runtime_ownership import ProfileOwnership
    ownership = ProfileOwnership()
    home = get_hermes_home()
    ownership.reserve([home])  # Native conflict refusal; never stop or replace an owner.
    handle = None
    carrier = None
    child = None
    runner = None
    signal_handlers = []
    try:
        from gateway.run import GatewayRunner, load_gateway_config_for_runner
        from gateway.session_authority import initialize_session_authority
        from gateway.run_api import start_gateway_api, stop_gateway_api
        runner = GatewayRunner(config=load_gateway_config_for_runner())
        await initialize_session_authority(runner, profile_id=str(home), instance_id=uuid.uuid4().hex)
        handle = await start_gateway_api(runner)
        await handle.app.ensure()
        from hermes_cli.web_server_chat import _build_gateway_ws_url
        from hermes_cli.gateway_client import connect_gateway
        from hermes_cli.gateway_session_store import _creation_policy, _launch_cwd
        # Native own-process credential stays in child environment, never diagnostics.
        endpoint = _build_gateway_ws_url()
        os.environ['HERMES_TUI_GATEWAY_URL'] = endpoint
        launch = vars(parsed)
        policy = _creation_policy(launch)
        async with connect_gateway() as client:
            created = await client.rpc('session.create', request_id=uuid.uuid4().hex, source='cli', cwd=_launch_cwd(), **policy)
        session_id = created['session_id']
        carrier = module.HermesCarrier(runner, session_id, prefix)
        env = dict(os.environ)
        import hermes_cli.main as native_main
        native_root = str(Path(native_main.__file__).resolve().parent.parent)
        child_code = "import sys;root,*args=sys.argv[1:];sys.path.insert(0,root);sys.argv=['hermes',*args];from hermes_cli.main import main;main()"
        command = [sys.executable, '-I', '-c', child_code, native_root, *args, '--resume', session_id]
        child = await asyncio.create_subprocess_exec(*command, env=env)
        loop = asyncio.get_running_loop()
        for signum in (signal.SIGINT, signal.SIGTERM):
            try:
                loop.add_signal_handler(signum, lambda number=signum: child.send_signal(number) if child.returncode is None else None)
                signal_handlers.append(signum)
            except NotImplementedError:
                pass  # Windows console delivery remains native; cleanup is still bounded.
        result = await child.wait()
        return 128 - result if result < 0 else result
    finally:
        try:
            try:
                if child is not None and child.returncode is None:
                    child.terminate()
                    try:
                        await asyncio.wait_for(child.wait(), timeout=5)
                    except asyncio.TimeoutError:
                        child.kill()
                        await asyncio.wait_for(child.wait(), timeout=5)
            finally:
                if carrier is not None:
                    carrier.detach()
        finally:
            try:
                try:
                    if handle is not None:
                        await asyncio.wait_for(stop_gateway_api(handle), timeout=10)
                finally:
                    if runner is not None:
                        await asyncio.wait_for(runner.stop(), timeout=20)
            finally:
                for signum in signal_handlers:
                    asyncio.get_running_loop().remove_signal_handler(signum)
                os.environ.pop('HERMES_TUI_GATEWAY_URL', None)
                ownership.release(home)


if __name__ == '__main__':
    try:
        sys.exit(asyncio.run(run_native_hermes(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:])))
    except Exception as error:
        # Only fixed categories, never native exception text or transport credentials.
        category = 'missing-runtime-module' if isinstance(error, ImportError) else 'cleanup-timeout' if isinstance(error, TimeoutError) else 'native-admission-or-startup'
        print(f'Vibe Hermes native bootstrap refused ({category}); check the selected installation or use read delivery', file=sys.stderr)
        sys.exit(1)
