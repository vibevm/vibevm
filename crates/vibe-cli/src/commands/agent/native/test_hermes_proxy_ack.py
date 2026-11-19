"""Closed invocation-owned proxy ACK; never imports native Hermes or providers."""
import importlib.util
import os
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "vibe_hermes_selection", Path(__file__).with_name("hermes_selection.py")
)
selection = importlib.util.module_from_spec(spec)
spec.loader.exec_module(selection)


class ProxyAckTests(unittest.TestCase):
    def route(self, **changes):
        return dict(kind="managed-px", endpoint="127.0.0.1:3128",
                    no_proxy="localhost,127.0.0.1,::1,.synthetic.test", **changes)

    def test_exact_route_is_child_scoped_and_preserves_local_bypass(self):
        route = self.route()
        with patch.dict(os.environ, {}, clear=True):
            selection.admit_route_ack({"route": route}, True)
            self.assertEqual(os.environ["HTTP_PROXY"], "http://127.0.0.1:3128")
            self.assertEqual(os.environ["HTTPS_PROXY"], os.environ["http_proxy"])
            self.assertEqual(os.environ["NO_PROXY"], route["no_proxy"])
            self.assertEqual(set(os.environ), {"HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy", "NO_PROXY", "no_proxy"})

    def test_invalid_route_cannot_publish_any_environment(self):
        good = self.route()
        bad = [None, {**good, "endpoint": "example.test:3128"},
               {**good, "endpoint": "user:synthetic@127.0.0.1:3128"},
               {**good, "endpoint": "127.0.0.1:3128/path"},
               {**good, "endpoint": "127.0.0.1:3128?x=1"},
               {**good, "endpoint": "127.0.0.1:0"},
               {**good, "endpoint": "127.0.0.1:3128\n"},
               {**good, "no_proxy": "*"},
               {**good, "no_proxy": "localhost,127.0.0.1,::1\n"},
               {**good, "env": {"SYNTHETIC_EXTRA": "not-allowed"}}]
        for route in bad:
            with self.subTest(route=route), patch.dict(os.environ, {}, clear=True):
                with self.assertRaises((ValueError, TypeError)):
                    selection.admit_route_ack({"route": route}, True)
                self.assertEqual(dict(os.environ), {})

    def test_old_no_route_ack_only_works_without_required_route(self):
        with patch.dict(os.environ, {}, clear=True):
            selection.admit_route_ack({}, False)
            self.assertEqual(dict(os.environ), {})
            with self.assertRaises(ValueError):
                selection.admit_route_ack({}, True)
            with self.assertRaises(ValueError):
                selection.admit_route_ack({"route": self.route()}, False)


if __name__ == "__main__":
    unittest.main()
