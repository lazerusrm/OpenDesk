#!/usr/bin/env python3
"""Synthetic contract tests for the RustDesk Pro snapshot exporter."""

import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "export-rustdesk-pro-sanitized.py"


class SanitizedExportTests(unittest.TestCase):
    def make_source_database(self, path: Path) -> None:
        owner_id = bytes.fromhex("01010101010101010101010101010101")
        member_id = bytes.fromhex("02020202020202020202020202020202")
        group_id = bytes.fromhex("03030303030303030303030303030303")
        target_group_id = bytes.fromhex("0b" * 16)
        peer_id = bytes.fromhex("04040404040404040404040404040404")
        personal_book_id = bytes.fromhex("05050505050505050505050505050505")
        shared_book_id = bytes.fromhex("06060606060606060606060606060606")

        connection = sqlite3.connect(path)
        connection.executescript(
            """
            CREATE TABLE "user" (guid BLOB, name TEXT, role INTEGER, grp BLOB);
            CREATE TABLE grp (guid BLOB, name TEXT, info TEXT);
            CREATE TABLE cross_grp (incoming BLOB, outgoing BLOB, type INTEGER);
            CREATE TABLE peer (guid BLOB, id TEXT, "user" BLOB, grp BLOB, info TEXT);
            CREATE TABLE ab (guid BLOB, name TEXT, owner BLOB, personal INTEGER);
            CREATE TABLE ab_rule (
                guid BLOB, ab BLOB, "user" BLOB, grp BLOB, rule INTEGER
            );
            CREATE TABLE ab_peer (
                guid BLOB, ab BLOB, peer BLOB, note TEXT, deleted_at TEXT, info TEXT
            );
            """
        )
        connection.executemany(
            "INSERT INTO grp (guid, name, info) VALUES (?, ?, ?)",
            [
                (group_id, "fixture-group", "{}"),
                (target_group_id, "fixture-target", '{"no_conn_in_group":1}'),
            ],
        )
        connection.execute(
            "INSERT INTO cross_grp (incoming, outgoing, type) VALUES (?, ?, 0)",
            (target_group_id, group_id),
        )
        connection.executemany(
            'INSERT INTO "user" (guid, name, role, grp) VALUES (?, ?, ?, ?)',
            [
                (owner_id, "fixture-owner", 1, group_id),
                (member_id, "fixture-member", 1, group_id),
            ],
        )
        connection.execute(
            'INSERT INTO peer (guid, id, "user", grp, info) VALUES (?, ?, ?, NULL, ?)',
            (peer_id, "fixture-device", owner_id, '{"device_name":"Fixture desktop"}'),
        )
        connection.executemany(
            "INSERT INTO ab (guid, name, owner, personal) VALUES (?, ?, ?, ?)",
            [
                (personal_book_id, "fixture-personal-book", owner_id, 1),
                (shared_book_id, "fixture-shared-book", owner_id, 0),
            ],
        )
        connection.executemany(
            'INSERT INTO ab_rule (guid, ab, "user", grp, rule) VALUES (?, ?, ?, ?, ?)',
            [
                (bytes.fromhex("07" * 16), personal_book_id, member_id, None, 1),
                (bytes.fromhex("08" * 16), shared_book_id, None, group_id, 2),
                (bytes.fromhex("09" * 16), shared_book_id, owner_id, None, 3),
            ],
        )
        connection.execute(
            "INSERT INTO ab_peer (guid, ab, peer, note, deleted_at, info) "
            "VALUES (?, ?, ?, ?, NULL, ?)",
            (
                bytes.fromhex("0a" * 16),
                personal_book_id,
                peer_id,
                "fixture note",
                '{"alias":"Front desk","hash":"excluded"}',
            ),
        )
        connection.commit()
        connection.close()

    def export(self, database: Path) -> dict[str, object]:
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--database",
                str(database),
                "--source-instance",
                "fixture-instance",
                "--source-export-id",
                "fixture-export",
                "--source-schema-version",
                "fixture-schema",
                "--role-map",
                "1:admin",
                "--retire-unsupported",
                "settings",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_exports_book_kinds_and_user_group_permissions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / "synthetic-source.sqlite"
            self.make_source_database(database)
            document = self.export(database)

        books = {book["book_kind"]: book for book in document["address_books"]}
        self.assertEqual(books["personal"]["name"], "Personal · fixture-owner")
        self.assertEqual(books["shared"]["name"], "fixture-shared-book")
        self.assertEqual(
            books["personal"]["rules"],
            [{
                "principal_type": "user",
                "principal_id": "02" * 16,
                "permission": "read",
            }],
        )
        self.assertEqual(
            books["shared"]["rules"],
            [
                {
                    "principal_type": "group",
                    "principal_id": "03" * 16,
                    "permission": "write",
                },
                {
                    "principal_type": "user",
                    "principal_id": "01" * 16,
                    "permission": "admin",
                },
            ],
        )
        groups = {group["name"]: group for group in document["groups"]}
        self.assertTrue(groups["fixture-group"]["allow_device_access_within_group"])
        self.assertFalse(groups["fixture-target"]["allow_device_access_within_group"])
        self.assertEqual(
            document["cross_group_access"],
            [{"source_group_id": "03" * 16, "target_group_id": "0b" * 16}],
        )
        self.assertEqual(document["devices"][0]["owner_source_user_id"], "01" * 16)
        self.assertEqual(document["devices"][0]["alias"], "Fixture desktop")
        self.assertEqual(document["address_book_entries"][0]["alias"], "Front desk")
        self.assertEqual(document["address_book_entries"][0]["notes"], "fixture note")


if __name__ == "__main__":
    unittest.main()
