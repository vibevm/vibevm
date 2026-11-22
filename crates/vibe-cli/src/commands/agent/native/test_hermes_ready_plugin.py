"""Offline contract checks; native transport/provider proof remains separate."""
import hashlib
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('ready_plugin', Path(__file__).with_name('hermes_ready_plugin.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ConversationContract(unittest.TestCase):
    def setUp(self):
        self.identity = {'profile': '/synthetic/profile', 'instance': 'owner', 'epoch': 1,
                         'runtime_protocol': 'hermes-gateway-v1', 'cwd': '/synthetic/project'}
        self.plugin = module.ConversationPlugin(self.snapshot, lambda sid: sid)

    def snapshot(self, sid):
        return {**self.identity, 'session': sid}

    def bind(self, text='project + personal', sid='history'):
        params = {'protocol': module.PROTOCOL, 'snapshot': self.snapshot(sid),
                  'scope': 'conversation', 'text': text,
                  'digest': hashlib.sha256(text.encode()).hexdigest(), 'project': 'project-node'}
        return self.plugin.register_contribution(params, session_id=sid)

    def request(self, sid='history', parent='', api_mode='chat_completions'):
        self.plugin.before_turn(session_id=sid, task_id='task', turn_id='turn', parent_session_id=parent)
        request = {'messages': [{'role': 'system', 'content': 'NATIVE'}, {'role': 'user', 'content': 'ask'}],
                   'tools': [{'name': 'native_tool'}]}
        result = self.plugin.request(request=request, session_id=sid, task_id='task', turn_id='turn', api_mode=api_mode)
        self.assertEqual(request['messages'][0]['content'], 'NATIVE')
        return result

    def test_two_sends_one_conversation_and_other_client(self):
        self.bind()
        for _ in range(3):
            result = self.request()
            self.assertEqual(result['request']['messages'][0]['content'], 'NATIVE\n\nproject + personal')
            self.assertEqual(result['request']['tools'], [{'name': 'native_tool'}])
        self.assertEqual(self.plugin.status({}, session_id='history')['observed_requests'], 3)

    def test_identical_idempotent_and_updated_digest_replaces(self):
        self.bind('OLD')
        self.bind('OLD')
        self.assertEqual(len(self.plugin.bindings), 1)
        self.bind('NEW')
        content = self.request()['request']['messages'][0]['content']
        self.assertEqual(content, 'NATIVE\n\nNEW')
        self.assertNotIn('OLD', content)

    def test_other_history_and_declared_child_unchanged(self):
        self.bind()
        self.assertIsNone(self.request('other'))
        self.assertIsNone(self.request(parent='parent'))

    def test_owner_or_workspace_drift_not_rebound(self):
        self.bind()
        self.identity['instance'] = 'replacement'
        self.assertIsNone(self.request())
        self.assertEqual(self.plugin.status({}, session_id='history')['last_reason'], 'identity-changed')

    def test_registration_rechecks_snapshot(self):
        snapshot = self.snapshot('history')
        self.identity['cwd'] = '/changed'
        with self.assertRaises(ValueError):
            self.plugin.register_contribution({'protocol': module.PROTOCOL, 'snapshot': snapshot,
                'scope': 'conversation', 'text': 'text', 'digest': hashlib.sha256(b'text').hexdigest(),
                'project': 'node'}, session_id='history')
        self.assertFalse(self.plugin.bindings)

    def test_no_unscoped_or_strict_registration(self):
        with self.assertRaises(ValueError):
            self.plugin.describe({}, session_id=None)
        self.assertFalse(self.plugin.describe({}, session_id='history')['strict'])
        with self.assertRaises(ValueError):
            self.plugin.register_contribution({'scope': 'strict'}, session_id='history')

    def test_native_rpc_envelope_session_checked(self):
        text = 'native-rpc'
        params = {'protocol': module.PROTOCOL, 'snapshot': self.snapshot('history'),
                  'scope': 'conversation', 'text': text, 'project': 'node',
                  'digest': hashlib.sha256(text.encode()).hexdigest(), 'session_id': 'history'}
        receipt = self.plugin.register_contribution(params, session_id='history')
        self.assertEqual(receipt['digest'], params['digest'])
        params['session_id'] = 'other'
        with self.assertRaises(ValueError):
            self.plugin.register_contribution(params, session_id='history')

    def test_no_path_read_rpc_or_digest_mismatch(self):
        with self.assertRaises(ValueError):
            self.plugin.register_contribution({'path': '/private/file'}, session_id='history')
        with self.assertRaises(ValueError):
            self.plugin.register_contribution({'protocol': module.PROTOCOL,
                'snapshot': self.snapshot('history'), 'scope': 'conversation', 'text': 'text',
                'digest': '0' * 64, 'project': 'node'}, session_id='history')
        self.assertFalse(self.plugin.bindings)

    def test_unsupported_provider_explicit_status_not_fake_delivery(self):
        self.bind()
        self.assertIsNone(self.request(api_mode='anthropic'))
        status = self.plugin.status({}, session_id='history')
        self.assertEqual(status['observed_requests'], 0)
        self.assertEqual(status['last_reason'], 'unsupported-request-shape')
        self.assertNotIn('text', status)

    def test_unload_loses_binding_not_false_persistence(self):
        self.bind()
        self.plugin = module.ConversationPlugin(self.snapshot, lambda sid: sid)
        self.assertIsNone(self.request())


if __name__ == '__main__':
    unittest.main()
