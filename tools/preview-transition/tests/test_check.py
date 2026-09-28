"""Thin request transport cases: no source selection or domain parsing."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("preview_check_transport", Path(__file__).parents[1] / "check.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class TransportTests(unittest.TestCase):
    def invoke(self, roots):
        argv = ["check.py", "--transition", "transition.json", "--mode", "documentation", "--stage", "intake"]
        for root in roots:
            argv.extend(["--repository-root", root])
        result = type("Result", (), {"stdout": b"", "stderr": b"", "returncode": 2})()
        with patch.object(CHECK.sys, "argv", argv), patch.object(CHECK.subprocess, "run", return_value=result) as run, patch.object(CHECK.sys, "stdout", io.TextIOWrapper(io.BytesIO())):
            self.assertEqual(CHECK.main(), 2)
            return json.loads(run.call_args.kwargs["input"])

    def test_absence_and_exact_repeated_values_preserved(self):
        self.assertNotIn("repository_roots", self.invoke([]))
        self.assertEqual(self.invoke(["docs=/tmp/space = suffix", "other=relative/../path"])["repository_roots"], {"docs": "/tmp/space = suffix", "other": "relative/../path"})

    def test_duplicate_and_missing_separator_refuse_before_subprocess(self):
        for roots in (["docs=/one", "docs=/two"], ["missing"]):
            argv = ["check.py", "--transition", "transition.json", "--mode", "documentation"]
            for root in roots:
                argv.extend(["--repository-root", root])
            with patch.object(CHECK.sys, "argv", argv), patch.object(CHECK.subprocess, "run") as run, contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
                CHECK.main()
            self.assertEqual(error.exception.code, 2)
            run.assert_not_called()

    def test_product_key_is_preserved_for_rust_admission(self):
        self.assertEqual(self.invoke(["product=unmodified"])["repository_roots"], {"product": "unmodified"})
