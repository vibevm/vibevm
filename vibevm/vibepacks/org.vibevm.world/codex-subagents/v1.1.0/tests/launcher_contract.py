"""Offline POSIX launcher contract: no login, network, or model spend."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

PACKAGE = Path(__file__).resolve().parents[1]
LAUNCHER = PACKAGE / "launchers/codex-worker"


class LauncherContract(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="codex-worker-contract-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.home = self.root / "separate profile"
        self.cli = self.root / "fake codex"
        self.cli.write_text(
            "#!/usr/bin/env python3\n"
            "import json,os,sys\n"
            "print(json.dumps({'args':sys.argv[1:],'cwd':os.getcwd(),"
            "'input':sys.stdin.read(),'home':os.environ.get('CODEX_HOME'),"
            "'identity_names':[k for k in os.environ if k.startswith(('CODEX_','OPENAI_')) "
            "and k not in ('CODEX_HOME','CODEX_CA_CERTIFICATE')]}))\n"
            "sys.exit(int(os.environ.get('FAKE_CODEX_EXIT','0')))\n"
        )
        self.cli.chmod(0o700)
        self.env = dict(os.environ)
        self.env.update(
            CODEX_WORKER_HOME=str(self.home), CODEX_WORKER_BIN=str(self.cli),
            OPENAI_API_KEY="sentinel", CODEX_ACCESS_TOKEN="sentinel",
            OPENAI_FEDERATION_RULE_ID="sentinel",
            OPENAI_IDENTITY_TOKEN_FILE="/nonexistent/sentinel",
            CODEX_SESSION_ID="sentinel", CODEX_APP_TOOLS_PIPE_PATH="sentinel",
        )

    def invoke(self, *args, text="", env=None):
        return subprocess.run(
            ["bash", str(LAUNCHER), *args], cwd=self.root,
            env=self.env if env is None else env, input=text,
            text=True, capture_output=True,
        )

    def test_quiet_exec_account_and_task_defaults(self):
        result = self.invoke("exec", "-C", "/a project", "-", text="packet bytes\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        observed = json.loads(result.stdout)
        self.assertEqual(observed["cwd"], str(self.root))
        self.assertEqual(observed["home"], str(self.home.resolve()))
        self.assertEqual(observed["identity_names"], [])
        self.assertEqual(observed["input"], "packet bytes\n")
        argv = observed["args"]
        for flag in ("--no-daemon", "--strict-config", "--ignore-user-config", "--json"):
            self.assertIn(flag, argv)
        self.assertIn("project_doc_max_bytes=0", argv)
        self.assertIn('cli_auth_credentials_store="file"', argv)
        self.assertIn('forced_login_method="chatgpt"', argv)
        self.assertIn('model_reasoning_effort="medium"', argv)
        self.assertEqual(argv[argv.index("--model") + 1], "gpt-6.1-sol")
        self.assertEqual(argv[-3:], ["-C", "/a project", "-"])

    def test_interactive_login_does_not_enable_quiet_exec(self):
        result = self.invoke("login", "--device-auth")
        self.assertEqual(result.returncode, 0, result.stderr)
        argv = json.loads(result.stdout)["args"]
        self.assertEqual(argv[-2:], ["login", "--device-auth"])
        self.assertNotIn("project_doc_max_bytes=0", argv)

    def test_status_uses_isolated_login(self):
        result = self.invoke("status")
        self.assertEqual(json.loads(result.stdout)["args"][-2:], ["login", "status"])

    def test_real_child_exit_is_preserved(self):
        env = dict(self.env, FAKE_CODEX_EXIT="37")
        self.assertEqual(self.invoke("exec", "a prompt", env=env).returncode, 37)

    def test_explicit_task_flags_replace_defaults_without_duplicates(self):
        result = self.invoke("exec", "-s", "read-only", "-m", "chosen-model", "--json", "--color", "always", "prompt")
        self.assertEqual(result.returncode, 0, result.stderr)
        argv = json.loads(result.stdout)["args"]
        self.assertNotIn("workspace-write", argv)
        self.assertNotIn("gpt-6.1-sol", argv)
        self.assertNotIn("--sandbox", argv)
        self.assertNotIn("--model", argv)
        self.assertEqual(argv.count("--json"), 1)
        self.assertEqual(argv.count("--color"), 1)

    def test_packet_prompt_empty_and_quoted_args_survive(self):
        tail = ["", "quote\" inside", "back\\slash", "a prompt with spaces"]
        result = self.invoke("exec", *tail)
        self.assertEqual(json.loads(result.stdout)["args"][-4:], tail)

    def test_option_terminator_keeps_literal_prompt_and_default_model(self):
        for prompt in ("-p literal prompt", "--model=literal prompt"):
            with self.subTest(prompt=prompt):
                result = self.invoke("exec", "--", prompt)
                self.assertEqual(result.returncode, 0, result.stderr)
                argv = json.loads(result.stdout)["args"]
                self.assertEqual(argv[-2:], ["--", prompt])
                self.assertEqual(argv[argv.index("--model") + 1], "gpt-6.1-sol")

    def test_boot_and_account_override_are_refused(self):
        for tail in (
            ["-c", "project_doc_max_bytes=50000"],
            ["--config=project_doc_max_bytes=50000"],
            ["-cproject_doc_max_bytes=50000"],
            ["-c", '\"project_doc_max_bytes\"=50000'],
            ["--profile", "primary"],
            ["-c", 'cli_auth_credentials_store="keyring"'],
        ):
            with self.subTest(tail=tail):
                result = self.invoke("exec", *tail)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")

    def test_explicit_absolute_home_is_required(self):
        for value in ("", "relative/profile"):
            with self.subTest(value=value):
                env = dict(self.env, CODEX_WORKER_HOME=value)
                self.assertEqual(self.invoke("exec", env=env).returncode, 2)

    def test_primary_home_is_refused_if_present(self):
        primary = Path.home() / ".codex"
        if not primary.is_dir():
            self.skipTest("default Codex directory absent")
        env = dict(self.env, CODEX_WORKER_HOME=str(primary))
        result = self.invoke("status", env=env)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
