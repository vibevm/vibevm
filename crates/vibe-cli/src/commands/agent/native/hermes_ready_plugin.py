"""Explicit conversation-scoped, best-effort Vibe contribution for a READY owner.

Operator prerequisite: install/enable this trusted plugin before starting Hermes.
This module never installs itself, starts an owner, or enforces permissions.
State lasts only for this loaded owner; unload/restart loses the contribution.
Native middleware is fail-open. A registration receipt is NOT delivery evidence.
"""
from collections import OrderedDict
from copy import deepcopy
import hashlib
from pathlib import Path
import threading

PROTOCOL = 'vibe.hermes.conversation.v1'
MAX_BYTES = 16 * 1024 * 1024
MAX_BINDINGS = 128
MAX_TURNS = 1024


def native_snapshot(session_id):
    """Called inside native session-authorized plugin RPC/profile scope."""
    from gateway.run_api import _ACTIVE_RUNNER
    from gateway.session_authorities import active_authority, require_serving
    from gateway.session_policy import policy_for_source
    from gateway.session_contract import CANONICAL_GATEWAY_PROTOCOL
    authority = active_authority(_ACTIVE_RUNNER)
    if authority is None:
        raise ValueError('owner unavailable')
    require_serving(authority)
    if getattr(authority, 'retiring', False):
        raise ValueError('owner draining')
    live = authority.sessions.get(session_id)
    if live is None or live.source is None:
        raise ValueError('exact live session required')
    policy = policy_for_source(authority.runner, live.source)
    if policy is None or not policy.cwd:
        raise ValueError('session workspace unavailable')
    cwd = Path(policy.cwd)
    if not cwd.is_absolute() or not cwd.is_dir():
        raise ValueError('session workspace unavailable')
    return {'profile': authority.profile_id, 'instance': authority.instance_id,
            'epoch': authority.epoch, 'runtime_protocol': CANONICAL_GATEWAY_PROTOCOL,
            'session': session_id, 'cwd': str(cwd.resolve())}


def request_conversation(session_id):
    """Resolve resident-agent aliases without following another conversation."""
    from gateway.run_api import _ACTIVE_RUNNER
    from gateway.session_authorities import active_authority
    from gateway.session_plugin_rpc import _live
    authority = active_authority(_ACTIVE_RUNNER)
    found = _live(authority, session_id) if authority is not None else None
    return found[0] if found is not None else None


class ConversationPlugin:
    def __init__(self, snapshot=native_snapshot, conversation=request_conversation):
        self.snapshot = snapshot
        self.conversation = conversation
        self.lock = threading.RLock()
        self.bindings = {}
        self.turns = OrderedDict()

    def describe(self, params, *, session_id, **_):
        self._session(session_id)
        return {'protocol': PROTOCOL, 'delivery': 'ready-session-best-effort',
                'strict': False, 'scope': 'conversation', 'max_bytes': MAX_BYTES,
                'snapshot': self.snapshot(session_id)}

    @staticmethod
    def _session(session_id):
        if not isinstance(session_id, str) or not session_id or len(session_id) > 512:
            raise ValueError('exact session required')

    def register_contribution(self, params, *, session_id, **_):
        self._session(session_id)
        params = dict(params)
        if params.pop('session_id', session_id) != session_id:
            raise ValueError('native session envelope mismatch')
        fields = {'protocol', 'snapshot', 'scope', 'text', 'digest', 'project'}
        if set(params) != fields or params['protocol'] != PROTOCOL:
            raise ValueError('unsupported contribution protocol')
        if params['scope'] != 'conversation':
            raise ValueError('explicit conversation scope required')
        text, digest, project = params['text'], params['digest'], params['project']
        if not isinstance(text, str) or not text:
            raise ValueError('nonempty UTF-8 contribution required')
        encoded = text.encode('utf-8', errors='strict')
        if len(encoded) > MAX_BYTES or hashlib.sha256(encoded).hexdigest() != digest:
            raise ValueError('contribution digest or size mismatch')
        if not isinstance(project, str) or not project or len(project) > 4096:
            raise ValueError('project identity required')
        with self.lock:
            current = self.snapshot(session_id)
            if params['snapshot'] != current:
                raise ValueError('owner or session workspace changed')
            previous = self.bindings.get(session_id)
            if previous is None and len(self.bindings) >= MAX_BINDINGS:
                raise ValueError('conversation binding capacity reached')
            # Explicit registration replaces one slot, never stacks instructions.
            self.bindings[session_id] = {'snapshot': current, 'text': text,
                                         'digest': digest, 'project': project,
                                         'observed': 0, 'last_reason': 'not-observed'}
            if previous is not None and previous['digest'] == digest and previous['project'] == project:
                self.bindings[session_id]['observed'] = previous['observed']
                self.bindings[session_id]['last_reason'] = previous['last_reason']
            return self._receipt(session_id)

    def status(self, params, *, session_id, **_):
        self._session(session_id)
        with self.lock:
            self.snapshot(session_id)
            return self._receipt(session_id)

    def _receipt(self, session_id):
        binding = self.bindings.get(session_id)
        return {'protocol': PROTOCOL, 'delivery': 'ready-session-best-effort',
                'strict': False, 'scope': 'conversation',
                'snapshot': binding['snapshot'] if binding else None,
                'digest': binding['digest'] if binding else None,
                'project': binding['project'] if binding else None,
                'observed_requests': binding['observed'] if binding else 0,
                'last_reason': binding['last_reason'] if binding else 'not-bound'}

    def before_turn(self, *, session_id='', task_id='', turn_id='', parent_session_id='', **_):
        # This filters observed ordinary main turns only. Native auxiliary/fork
        # and compaction coverage is not a universal worker-exclusion guarantee.
        if not all(isinstance(value, str) and value for value in (session_id, task_id, turn_id)):
            return None
        with self.lock:
            key = (session_id, task_id, turn_id)
            self.turns[key] = not bool(parent_session_id)
            self.turns.move_to_end(key)
            while len(self.turns) > MAX_TURNS:
                self.turns.popitem(last=False)
        return None  # Never inject user content or veto ordinary native work.

    def request(self, *, request, session_id='', task_id='', turn_id='', api_mode='', **_):
        with self.lock:
            if not self.turns.get((session_id, task_id, turn_id), False):
                return None
            sid = self.conversation(session_id)
            binding = self.bindings.get(sid)
            if binding is None:
                return None
            if self.snapshot(sid) != binding['snapshot']:
                binding['last_reason'] = 'identity-changed'
                return None
            # v1 deliberately admits only the confirmed chat-completions text
            # system representation. No generic provider/cache-shape converter.
            messages = request.get('messages')
            if api_mode != 'chat_completions' or not isinstance(messages, list):
                binding['last_reason'] = 'unsupported-request-shape'
                return None
            system = [row for row in messages if isinstance(row, dict) and row.get('role') == 'system']
            if len(system) != 1 or not isinstance(system[0].get('content'), str):
                binding['last_reason'] = 'unsupported-system-shape'
                return None
            rewritten = deepcopy(request)
            for row in rewritten['messages']:
                if isinstance(row, dict) and row.get('role') == 'system':
                    row['content'] += '\n\n' + binding['text']
                    break
            binding['observed'] += 1
            binding['last_reason'] = 'middleware-rewrite-observed'
            return {'request': rewritten}


def register(ctx):
    plugin = ConversationPlugin()
    ctx.register_rpc('describe', plugin.describe)
    ctx.register_rpc('register', plugin.register_contribution)
    ctx.register_rpc('status', plugin.status)
    ctx.register_hook('pre_llm_call', plugin.before_turn)
    ctx.register_middleware('llm_request', plugin.request)
