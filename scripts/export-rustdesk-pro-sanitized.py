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

SCHEMA_VERSION = 2
NAMESPACE = uuid.UUID("3f1e9cbe-d5d4-4618-ae5a-9a391e42635a")
UNSUPPORTED_TABLES = (
    "control_role_map",
    "custom_client",
    "role_scope",
    "strategy",
    "user_roles",
)
RETIRABLE_UNSUPPORTED = {"custom_client", "strategy", "settings"}


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
    assignments = tuple(f"{name}" + "=" for name in ("password", "token", "secret"))
    if (
        "-----begin" in lowered
        or lowered.startswith("$2a$")
        or lowered.startswith("$2b$")
        or lowered.startswith("$argon2")
        or any(assignment in lowered for assignment in assignments)
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


def unsupported_inventory(connection: sqlite3.Connection) -> dict[str, int]:
    inventory = {
        table: table_count(connection, table)
        for table in UNSUPPORTED_TABLES
        if table_exists(connection, table) and table_count(connection, table)
    }
    if table_exists(connection, "settings") and table_count(connection, "settings"):
        inventory["settings"] = table_count(connection, "settings")
    return inventory


def require_supported_source(connection: sqlite3.Connection, retired: set[str]) -> list[dict[str, object]]:
    inventory = unsupported_inventory(connection)
    unretired = set(inventory) - retired
    if unretired:
        fail(f"source has populated unsupported semantics: {next(iter(sorted(unretired)))}")
    return [
        {"category": category, "count": inventory[category], "disposition": "retired"}
        for category in sorted(inventory)
    ]


def export_snapshot(connection: sqlite3.Connection, role_map: dict[int, str], source_instance: str,
                    source_export_id: str, source_schema_version: str,
                    retired: set[str]) -> dict[str, object]:
    unsupported_semantics = require_supported_source(connection, retired)
    users = []
    memberships = []
    groups: dict[str, dict[str, object]] = {}
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
    for guid, name, info in connection.execute("SELECT guid, name, info FROM grp ORDER BY guid"):
        group_id = hex_id(guid)
        try:
            group_info = json.loads(info or "{}")
        except json.JSONDecodeError:
            fail("source group metadata is invalid")
        if not isinstance(group_info, dict) or set(group_info) - {"no_conn_in_group"}:
            fail("source group metadata requires an explicit reviewed mapping")
        no_conn_in_group = group_info.get("no_conn_in_group", 0)
        if no_conn_in_group not in (0, 1):
            fail("source group has an invalid within-group access policy")
        groups[group_id] = {
            "source_group_id": group_id,
            "name": text(name, "group name"),
            "allow_device_access_within_group": no_conn_in_group == 0,
        }
    if any(item["source_group_id"] not in groups for item in memberships):
        fail("source user references a missing group")
    known_user_ids = {item["source_user_id"] for item in users}

    cross_group_access = []
    if table_exists(connection, "cross_grp"):
        for incoming, outgoing, rule_type in connection.execute(
            "SELECT incoming, outgoing, type FROM cross_grp ORDER BY incoming, outgoing"
        ):
            source_group_id = hex_id(outgoing)
            target_group_id = hex_id(incoming)
            if rule_type != 0:
                fail("source cross-group rule requires an explicit reviewed mapping")
            if source_group_id not in groups or target_group_id not in groups:
                fail("source cross-group rule references a missing group")
            if source_group_id == target_group_id:
                fail("source cross-group rule references the same group")
            cross_group_access.append({
                "source_group_id": source_group_id,
                "target_group_id": target_group_id,
            })

    devices = []
    device_ids: set[bytes] = set()
    for guid, rustdesk_id, owner_guid, group_guid, info in connection.execute(
        'SELECT guid, id, "user", grp, info FROM peer ORDER BY guid'
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
        owner_id = owner_guid.hex() if isinstance(owner_guid, bytes) else None
        if owner_id is not None and owner_id not in known_user_ids:
            fail("source peer references a missing owner")
        devices.append({
            "rustdesk_id": text(rustdesk_id, "RustDesk ID"),
            "alias": text(device_name, "device alias"),
            "hostname": None,
            "owner_source_user_id": owner_id,
            "source_group_ids": [],
        })

    books = []
    known_books: set[bytes] = set()
    for guid, name, owner, personal in connection.execute(
        "SELECT guid, name, owner, personal FROM ab ORDER BY guid"
    ):
        book_guid = bytes(guid) if isinstance(guid, bytes) else None
        owner_guid = bytes(owner) if isinstance(owner, bytes) else None
        if book_guid is None or owner_guid is None or personal not in (0, 1):
            fail("source contains an ownerless or invalid address book")
        known_books.add(book_guid)
        owner_id = owner_guid.hex()
        if owner_id not in known_user_ids:
            fail("source address book references a missing owner")
        owner_name = next(item["username"] for item in users if item["source_user_id"] == owner_id)
        display_name = f"Personal · {owner_name}" if personal == 1 else text(name, "address book name")
        rules = []
        for rule_user, rule_group, permission in connection.execute(
            'SELECT "user", grp, rule FROM ab_rule WHERE ab = ? ORDER BY guid',
            (book_guid,),
        ):
            if (rule_user is None) == (rule_group is None) or permission not in (1, 2, 3):
                fail("source address book contains an invalid access rule")
            principal_type = "user" if rule_user is not None else "group"
            principal = bytes(rule_user if rule_user is not None else rule_group)
            principal_id = principal.hex()
            if (principal_type == "user" and principal_id not in known_user_ids) or (
                principal_type == "group" and principal_id not in groups
            ):
                fail("source address book access rule references a missing principal")
            rules.append({
                "principal_type": principal_type,
                "principal_id": principal_id,
                "permission": {1: "read", 2: "write", 3: "admin"}[permission],
            })
        books.append({
            "source_address_book_id": book_guid.hex(),
            "name": display_name,
            "owner_source_user_id": owner_id,
            "book_kind": "personal" if personal == 1 else "shared",
            "rules": rules,
        })

    entries = []
    for book_guid, peer_guid, note, info in connection.execute(
        "SELECT ab, peer, note, info FROM ab_peer WHERE deleted_at IS NULL ORDER BY guid"
    ):
        book = bytes(book_guid) if isinstance(book_guid, bytes) else None
        peer = bytes(peer_guid) if isinstance(peer_guid, bytes) else None
        if book not in known_books or peer not in device_ids:
            fail("source address book link is not referentially complete")
        try:
            entry_info = json.loads(info or "{}")
        except json.JSONDecodeError:
            fail("source address-book entry metadata is invalid")
        if not isinstance(entry_info, dict) or set(entry_info) - {"alias", "hash"}:
            fail("source address-book entry metadata requires an explicit reviewed mapping")
        alias = entry_info.get("alias")
        if alias is not None and not isinstance(alias, str):
            fail("source address-book entry alias is invalid")
        rustdesk_id = connection.execute("SELECT id FROM peer WHERE guid = ?", (peer,)).fetchone()
        if rustdesk_id is None:
            fail("source address book link references a missing device")
        identifier = text(rustdesk_id[0], "RustDesk ID")
        entries.append({
            "source_address_book_id": book.hex(),
            "rustdesk_id": identifier,
            "alias": text(alias, "address book entry alias") if alias and alias.strip() else identifier,
            "notes": text(note, "address book entry notes") if note and note.strip() else None,
            "credential_reset_required": True,
        })

    if table_exists(connection, "settings") and table_count(connection, "settings") and "settings" not in retired:
        fail("source settings require an explicit reviewed mapping")

    snapshot = {
        "users": users,
        "groups": list(groups.values()),
        "user_group_memberships": memberships,
        "cross_group_access": cross_group_access,
        "devices": devices,
        "address_books": books,
        "address_book_entries": entries,
        "settings": [],
        "unsupported_semantics": unsupported_semantics,
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
    parser.add_argument("--retire-unsupported", action="append", default=[])
    parser.add_argument("--unsupported-inventory", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not args.database.is_file():
        fail("source database is not a regular file")
    role_map = source_role_map(args.role_map)
    retired = set(args.retire_unsupported)
    if not retired.issubset(RETIRABLE_UNSUPPORTED):
        fail("unsupported retirement category")
    uri = f"file:{args.database.resolve().as_posix()}?mode=ro&immutable=1"
    try:
        connection = sqlite3.connect(uri, uri=True)
        connection.execute("PRAGMA query_only = ON")
        if args.unsupported_inventory:
            print(json.dumps({"unsupported_semantics": unsupported_inventory(connection)},
                             separators=(",", ":"), sort_keys=True))
            return
        document = export_snapshot(
            connection, role_map, text(args.source_instance, "source instance"),
            text(args.source_export_id, "source export ID"),
            text(args.source_schema_version, "source schema version"), retired,
        )
    except (json.JSONDecodeError, sqlite3.Error):
        fail("source database cannot be read safely")
    print(json.dumps(document, separators=(",", ":"), sort_keys=True))


if __name__ == "__main__":
    main()
