#!/usr/bin/env python3
"""Integration tests for the protected RustDesk Pro credential exporter."""
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("export-rustdesk-pro-credentials.py")
SYNTHETIC_PASSWORD = "synthetic-migration-password"
SYNTHETIC_BCRYPT = "$2b$06$f2T.tThVDBYPF84d8/3aFO6FBVakCPTrYggGuOPfe.9hJ1m2srbMO"
RUN_ID = "11111111-1111-4111-8111-111111111111"
TARGET_ID = "22222222-2222-4222-8222-222222222222"


class CredentialExporterTests(unittest.TestCase):
    def source_database(self, verifier=SYNTHETIC_BCRYPT):
        database = Path(self.temp.name) / "source.sqlite"
        connection = sqlite3.connect(database)
        connection.execute('CREATE TABLE "user" (guid BLOB, password TEXT, status INTEGER)')
        connection.execute(
            'INSERT INTO "user" VALUES (?, ?, ?)',
            (bytes.fromhex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), verifier, 1),
        )
        connection.commit()
        connection.close()
        return database

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.input_path = Path(self.temp.name) / "sanitized-export.json"
        self.input_path.write_bytes(b'{"synthetic":true}')
        self.output_path = Path(self.temp.name) / "credentials.json"
        self.output_path.touch(mode=0o600)
        os.chmod(self.output_path, 0o600)

    def command(self, database):
        return [
            sys.executable, str(SCRIPT), "--database", str(database),
            "--input", str(self.input_path), "--output", str(self.output_path),
            "--source-instance", "synthetic-source", "--source-export-id", "synthetic-export",
            "--run-id", RUN_ID, "--target-instance-uuid", TARGET_ID,
        ]

    def test_success_writes_protected_artifact_without_secret_logging(self):
        result = subprocess.run(self.command(self.source_database()), capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")
        self.assertNotIn(SYNTHETIC_PASSWORD, result.stdout + result.stderr)
        artifact = json.loads(self.output_path.read_text())
        self.assertEqual(artifact["records"][0]["verifier"], SYNTHETIC_BCRYPT)
        self.assertNotIn(SYNTHETIC_PASSWORD, self.output_path.read_text())
        self.assertEqual(self.output_path.stat().st_mode & 0o777, 0o600)

    def test_invalid_verifier_fails_closed_without_secret_logging(self):
        before = self.output_path.read_bytes()
        result = subprocess.run(
            self.command(self.source_database("not-a-bcrypt-verifier")),
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertNotIn(SYNTHETIC_PASSWORD, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stderr)["category"], "source")
        self.assertEqual(self.output_path.read_bytes(), before)

    def test_output_must_be_precreated_mode_0600(self):
        self.output_path.unlink()
        result = subprocess.run(self.command(self.source_database()), capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertNotIn(SYNTHETIC_PASSWORD, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stderr)["category"], "source")


if __name__ == "__main__":
    unittest.main()
