#!/usr/bin/env python3
"""Export a read-only RustDesk Pro SQLite snapshot into OpenDesk's sanitized contract.

This is an external integration boundary. It never exports passwords, TFA,
sessions, free-form metadata, settings values, keys, or tokens. Unsupported
populated Pro structures stop the export rather than being silently weakened.
"""

import argparse
import hashlib
import json
import sqlite3
import sys
import uuid
from datetime import UTC, datetime
from pathlib import Path

SCHEMA_VERSION = 1
NAMESPACE = uuid.UUID("3f1e9cbe-d5d4-4618-ae5a-9a391e42635a")
UNSUPPORTED_TABLES = (
    "ab_rule",
    "control_role_map",
    "custom_client",
    "role_scope",
    "strategy",
    "user_roles",
)


def fail(message: str) -> None:
    print(json.dumps({"error": message}), file=sys.stderr)
    raise SystemExit(2)


def hex_id(value: bytes | None) -> str:
    if not isinstance(value, bytes) or not value:
        fail("source contains an invalid binary identifier")
    return value.hex()


def text(value: object, field: str) -> str:
    if not isinstance(value, str) or not value.strip() or any(c in value for c in "\r\n\t\x00"):
        fail(f"source contains an invalid {field}")
    lowered = value.lower()
    if (
        "-----begin" in lowered
        or lowered.startswith("$2a$")
        or lowered.startswith("$2b$")
        or lowered.startswith("$argon2")
        or "password=" in lowered
        or "token=" in lowered
        or "secret=" in lowered
        or "private_key" in lowered
    ):
        fail(f"source contains sensitive content in {field}")
    return value


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, separators=(",", ":"), sort_keys=True).encode("utf-8")


def table_exists(connection: sqlite3.Connection, table: str) -> bool:
    return connection.execute(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?", (table,)
    ).fetchone() is not None


def table_count(connection: sqlite3.Connection, table: str) -> int:
    return int(connection.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0])


def source_role_map(values: list[str]) -> dict[int, str]:
    result: dict[int, str] = {}
    for value in values:
        try:
            source, target = value.split(":", 1)
            source_role = int(source)
        except ValueError:
            fail("role mappings must be SOURCE_ROLE:admin|operator|read_only")
        if target not in {"admin", "operator", "read_only"} or source_role in result:
            fail("role mappings must be unique canonical roles")
        result[source_role] = target
    return result


def require_supported_source(connection: sqlite3.Connection) -> None:
    for table in UNSUPPORTED_TABLES:
        if table_exists(connection, table) and table_count(connection, table):
            fail(f"source has populated unsupported semantics: {table}")


def export_snapshot(connection: sqlite3.Connection, role_map: dict[int, str], source_instance: str,
                    source_export_id: str, source_schema_version: str,
                    settings_disposition: str) -> dict[str, object]:
    require_supported_source(connection)
    users = []
    memberships = []
    groups: dict[str, dict[str, str]] = {}
    for guid, name, role, group_guid in connection.execute(
        'SELECT guid, name, role, grp FROM "user" ORDER BY guid'
    ):
        if not isinstance(role, int) or role not in role_map:
            fail("source has an unmapped user role")
        user_id = hex_id(guid)
        group_id = hex_id(group_guid)
        users.append({
            "source_user_id": user_id,
            "username": text(name, "username"),
            "role": role_map[role],
            "credential_reset_required": True,
        })
        memberships.append({"source_user_id": user_id, "source_group_id": group_id})
    for guid, name in connection.execute("SELECT guid, name FROM grp ORDER BY guid"):
        group_id = hex_id(guid)
        groups[group_id] = {"source_group_id": group_id, "name": text(name, "group name")}
    if any(item["source_group_id"] not in groups for item in memberships):
        fail("source user references a missing group")

    devices = []
    device_ids: set[bytes] = set()
    for guid, rustdesk_id, group_guid, info in connection.execute(
        "SELECT guid, id, grp, info FROM peer ORDER BY guid"
    ):
        if group_guid is not None:
            fail("source has populated peer group semantics without a reviewed mapping")
        if info not in (None, "{}"):
            parsed_info = json.loads(info)
            if not isinstance(parsed_info, dict):
                fail("source peer metadata requires an explicit reviewed mapping")
        else:
            parsed_info = {}
        device_guid = bytes(guid) if isinstance(guid, bytes) else None
        if device_guid is None:
            fail("source contains an invalid peer identifier")
        device_ids.add(device_guid)
        device_name = parsed_info.get("device_name", rustdesk_id)
        if not isinstance(device_name, str):
            fail("source peer metadata contains an invalid device name")
        devices.append({
            "rustdesk_id": text(rustdesk_id, "RustDesk ID"),
            "alias": text(device_name, "device alias"),
            "hostname": None,
            "source_group_ids": [],
        })

    books = []
    known_books: set[bytes] = set()
    for guid, name, owner in connection.execute("SELECT guid, name, owner FROM ab ORDER BY guid"):
        book_guid = bytes(guid) if isinstance(guid, bytes) else None
        owner_guid = bytes(owner) if isinstance(owner, bytes) else None
        if book_guid is None or owner_guid is None:
            fail("source contains an ownerless or invalid address book")
        known_books.add(book_guid)
        owner_id = owner_guid.hex()
        if owner_id not in {item["source_user_id"] for item in users}:
            fail("source address book references a missing owner")
        books.append({
            "source_address_book_id": book_guid.hex(),
            "name": text(name, "address book name"),
            "owner_source_user_id": owner_id,
            "rules": [],
        })

    entries = []
    for book_guid, peer_guid, note in connection.execute(
        "SELECT ab, peer, note FROM ab_peer WHERE deleted_at IS NULL ORDER BY guid"
    ):
        book = bytes(book_guid) if isinstance(book_guid, bytes) else None
        peer = bytes(peer_guid) if isinstance(peer_guid, bytes) else None
        if book not in known_books or peer not in device_ids:
            fail("source address book link is not referentially complete")
        if note not in (None, ""):
            fail("source address book notes require an explicit reviewed mapping")
        rustdesk_id = connection.execute("SELECT id FROM peer WHERE guid = ?", (peer,)).fetchone()
        if rustdesk_id is None:
            fail("source address book link references a missing device")
        identifier = text(rustdesk_id[0], "RustDesk ID")
        entries.append({
            "source_address_book_id": book.hex(),
            "rustdesk_id": identifier,
            "alias": identifier,
            "notes": None,
            "credential_reset_required": True,
        })

    settings = []
    if table_exists(connection, "settings"):
        for (key,) in connection.execute("SELECT key FROM settings ORDER BY key"):
            settings.append({"key": text(key, "setting key"), "disposition": settings_disposition})

    snapshot = {
        "users": users,
        "groups": list(groups.values()),
        "user_group_memberships": memberships,
        "devices": devices,
        "address_books": books,
        "address_book_entries": entries,
        "settings": settings,
    }
    snapshot_sha256 = hashlib.sha256(canonical_bytes(snapshot)).hexdigest()
    now = datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z")
    run_id = uuid.uuid5(NAMESPACE, f"{source_instance}:{source_export_id}:{snapshot_sha256}")
    return {
        "schema_version": SCHEMA_VERSION,
        "provenance": {
            "source_system": "rustdesk_server_pro",
            "source_instance": source_instance,
            "source_export_id": source_export_id,
            "source_schema_version": source_schema_version,
            "exported_at": now,
            "snapshot_sha256": snapshot_sha256,
        },
        "run": {
            "run_id": str(run_id),
            "status": "exported",
            "started_at": now,
            "completed_at": now,
            "source_snapshot_sha256": snapshot_sha256,
        },
        **snapshot,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument("--source-instance", required=True)
    parser.add_argument("--source-export-id", required=True)
    parser.add_argument("--source-schema-version", required=True)
    parser.add_argument("--role-map", action="append", default=[])
    parser.add_argument(
        "--settings-disposition",
        choices=("exclude", "manual_review", "map", "retire"),
        default="manual_review",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not args.database.is_file():
        fail("source database is not a regular file")
    role_map = source_role_map(args.role_map)
    uri = f"file:{args.database.resolve().as_posix()}?mode=ro&immutable=1"
    try:
        connection = sqlite3.connect(uri, uri=True)
        connection.execute("PRAGMA query_only = ON")
        document = export_snapshot(
            connection, role_map, text(args.source_instance, "source instance"),
            text(args.source_export_id, "source export ID"),
            text(args.source_schema_version, "source schema version"), args.settings_disposition,
        )
    except (json.JSONDecodeError, sqlite3.Error):
        fail("source database cannot be read safely")
    print(json.dumps(document, separators=(",", ":"), sort_keys=True))


if __name__ == "__main__":
    main()
