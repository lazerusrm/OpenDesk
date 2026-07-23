#!/usr/bin/env python3
"""Read-only Pro credential export to a pre-created 0600 regular file."""
import argparse
import hashlib
import json
import os
import sqlite3
import stat
import sys
from pathlib import Path

UUID_RE = __import__("re").compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
)
BCRYPT_RE = __import__("re").compile(r"^\$2b\$06\$[./A-Za-z0-9]{53}$")
MAX_INPUT_BYTES = 16 * 1024 * 1024


def fail(category):
    print(json.dumps({"error": "credential export failed", "category": category}), file=sys.stderr)
    raise SystemExit(2)


def text(value):
    if not isinstance(value, str) or not value or any(c in value for c in "\r\n\t\x00"):
        fail("source")
    return value


def open_regular(path, flags):
    try:
        fd = os.open(path, flags | os.O_NOFOLLOW | os.O_CLOEXEC)
        info = os.fstat(fd)
    except OSError:
        fail("source")
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != os.getuid():
        os.close(fd)
        fail("permissions")
    return fd, info


def read_fd(fd, limit=None):
    chunks = []
    total = 0
    while chunk := os.read(fd, 1024 * 1024):
        total += len(chunk)
        if limit is not None and total > limit:
            fail("source")
        chunks.append(chunk)
    return b"".join(chunks)


def sidecars_are_safe(database):
    for suffix in ("-wal", "-shm"):
        sidecar = Path(f"{database}{suffix}")
        try:
            info = os.lstat(sidecar)
        except FileNotFoundError:
            continue
        except OSError:
            return False
        # Any directory entry, including a broken symlink, is unsafe.
        if stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
            return False
        if info.st_size != 0:
            return False
    return True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-instance", required=True)
    parser.add_argument("--source-export-id", required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--target-instance-uuid", required=True)
    args = parser.parse_args()
    if not UUID_RE.fullmatch(args.run_id) or not UUID_RE.fullmatch(args.target_instance_uuid):
        fail("arguments")
    if str(args.output) == "-" or not sidecars_are_safe(args.database):
        fail("source")

    input_fd = database_fd = output_fd = None
    try:
        input_fd, input_info = open_regular(args.input, os.O_RDONLY)
        input_bytes = read_fd(input_fd, MAX_INPUT_BYTES)
        os.close(input_fd)
        input_fd = None

        database_fd, _ = open_regular(args.database, os.O_RDONLY)
        # Keep the verified descriptor open while SQLite reads it.
        database_uri = f"file:/proc/self/fd/{database_fd}?mode=ro&immutable=1"
        connection = sqlite3.connect(database_uri, uri=True)
        try:
            connection.execute("PRAGMA query_only=ON")
            rows = connection.execute('SELECT guid, password FROM "user" ORDER BY guid').fetchall()
        finally:
            connection.close()
        os.close(database_fd)
        database_fd = None

        records = []
        for guid, verifier in rows:
            if not isinstance(guid, bytes) or not guid or not isinstance(verifier, str) or not BCRYPT_RE.fullmatch(verifier):
                fail("source")
            records.append({"source_user_id": guid.hex(), "verifier_algorithm": "bcrypt", "verifier": verifier})
        if not records:
            fail("source")
        artifact = {
            "schema_version": 1,
            "source_system": "rustdesk_server_pro",
            "source_instance": text(args.source_instance),
            "source_export_id": text(args.source_export_id),
            "run_id": args.run_id,
            "target_instance_uuid": args.target_instance_uuid,
            "input_sha256": hashlib.sha256(input_bytes).hexdigest(),
            "activation_policy": "activate_all",
            "records": records,
        }
        data = (json.dumps(artifact, separators=(",", ":"), sort_keys=True) + "\n").encode()

        output_fd, output_info = open_regular(args.output, os.O_WRONLY)
        if output_info.st_mode & 0o777 != 0o600:
            fail("permissions")
        os.ftruncate(output_fd, 0)
        os.lseek(output_fd, 0, os.SEEK_SET)
        view = memoryview(data)
        while view:
            view = view[os.write(output_fd, view):]
        os.fsync(output_fd)
    except (OSError, sqlite3.Error):
        fail("source")
    finally:
        for fd in (input_fd, database_fd, output_fd):
            if fd is not None:
                try:
                    os.close(fd)
                except OSError:
                    pass


if __name__ == "__main__":
    main()
