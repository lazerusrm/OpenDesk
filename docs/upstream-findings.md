# Upstream Findings

Local reference clones live under ignored `upstream/`:

- `upstream/rustdesk`
- `upstream/rustdesk-server`

These are working references only. Do not commit vendored upstream source into this repository unless the licensing and repository strategy are intentionally changed.

## Snapshot

Captured on initial planning pass:

- `rustdesk` HEAD: `58ee593e26d7e4251eb86dfc55878dc884044537`
- `rustdesk-server` HEAD: `815c728837b8a091c9feeeabb423d543be3a7f8d`

Both upstream repos are AGPL-licensed. OpenDesk should stay clean-room unless we intentionally fork or link against upstream code and accept the license implications.

## RustDesk Server OSS

The OSS server repository builds three binaries:

- `hbbs`: ID/rendezvous server
- `hbbr`: relay server
- `rustdesk-utils`: utility commands

The upstream Docker Compose example exposes:

- `21115/tcp`
- `21116/tcp`
- `21116/udp`
- `21117/tcp`
- `21118/tcp`
- `21119/tcp`

Relevant operational behavior:

- `hbbs` can be started with relay information such as `hbbs -r rustdesk.example.com:21117`.
- Server keys are stored as `id_ed25519` and `id_ed25519.pub` in the server data path.
- `rustdesk-utils` includes keypair generation/validation and a `doctor` check that covers expected ports.

OpenDesk should wrap, monitor, and document this stack rather than modify it during the initial control-plane build.

## RustDesk Client Hooks

The official client contains custom server configuration parsing in `src/custom_server.rs`.

The visible filename form supports:

```text
rustdesk-host=server.example.net.exe
rustdesk-host=server.example.net,key=PUBLIC_KEY,relay=server.example.net.exe
rustdesk-host=server.example.net,api=https://api.example.net,key=PUBLIC_KEY.exe
```

Important caveat:

This confirms the parser exists in current upstream source. It does not prove every official release, installer type, OS, or package format applies it the way we want. Client delivery must validate this on real Windows/Linux/macOS packages before we rely on it.

The client also has a deployment path:

```text
POST {api-server}/api/devices/deploy
Authorization: Bearer <token>
body: {"id": "...", "uuid": "...", "pk": "..."}
```

Expected response includes a `result` field such as `OK`. This looks useful for optional native-ish enrollment, but it should be treated as an upstream compatibility hypothesis until validated.

## Connection Shortcut Hook

RustDesk help text indicates a remote ID can include server and key context:

```text
DEVICE_ID@server.example.net:21117?key=PUBLIC_KEY
```

This may let OpenDesk generate connect strings for devices on non-default servers. For the primary OpenDesk workflow, we should still prefer normal IDs once the client is configured to the correct server.

## Replacement Implications

Prioritize these delivery paths in order:

1. Generate OS-specific scripts that install official clients and set options/config.
2. Validate filename-based custom server config for Windows portable/installer flows.
3. Validate RustDesk local config/import behavior per OS.
4. Implement OpenDesk's own endpoint registration API.
5. Optionally implement a compatibility endpoint for `/api/devices/deploy` if official clients can use it without a fork.

Do not build a RustDesk Pro API clone first. Add compatible API behavior only when it directly satisfies a full-replacement requirement that cannot be covered more cleanly through OpenDesk's own API, endpoint agent, or generated configuration.
