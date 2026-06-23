# Software Stack

This document locks the default implementation stack for OpenDesk. Changes require an ADR update.

## Backend

| Layer | Choice | Notes |
|---|---|---|
| Language | Rust | Default implementation language for OpenDesk service code. |
| HTTP framework | Axum | Small, typed, Tower-based, good fit for APIs and server-rendered pages. |
| Async runtime | Tokio | Required by Axum and common Rust service libraries. |
| Middleware | Tower / tower-http | Request tracing, compression, timeouts, auth layers, static file serving. |
| Templates | Askama | Compile-time checked server-rendered templates. |
| Serialization | Serde | JSON and typed config rendering. |
| Validation | garde or validator | Pick one during scaffold; use typed domain validators, not ad hoc strings. |
| Errors | thiserror plus anyhow at process boundaries | Domain errors stay typed; anyhow only for CLI/startup glue. |
| Logging/tracing | tracing / tracing-subscriber | Structured logs with redaction discipline. |

## Data

| Layer | Choice | Notes |
|---|---|---|
| Primary database | SQLite | First production target can remain SQLite if concurrency and backup tests pass. |
| Migration path | Postgres | Keep schema/query discipline compatible where practical. |
| SQL access | sqlx | Typed async SQL, migrations, SQLite/Postgres support. |
| IDs | UUIDv7 | Internal stable IDs; RustDesk IDs remain separate `rustdesk_id` fields. |
| Time | time crate | Store UTC timestamps. |

## Auth And Security

| Capability | Choice | Notes |
|---|---|---|
| Password hashing | Argon2id | For local accounts if enabled. |
| Sessions | Secure HTTP-only cookies | SameSite, expiry, CSRF strategy required. |
| Passkeys | webauthn-rs | Optional OpenDesk login hardening, especially phone passkeys. |
| OIDC | Later integration | Add only when required; reverse proxy OIDC remains acceptable. |
| Secrets | External secret manager references | ADR-008 governs managed access secrets. |

## Frontend

| Layer | Choice | Notes |
|---|---|---|
| Initial UI | Server-rendered HTML | Quiet operational dashboard, not SPA by default. |
| Progressive enhancement | htmx if needed | Use only when it simplifies CRUD/dashboard interactions. |
| CSS | Plain CSS or Tailwind | Choose at scaffold; keep design restrained and maintainable. |
| Rich client code | TypeScript/Vite only when justified | Required only for workflows that server-rendered UI cannot keep clean. |

## Client And Deployment Tooling

| Target | Choice | Notes |
|---|---|---|
| Windows endpoint scripts | PowerShell | Generated from typed Rust config structs. |
| Linux endpoint scripts | POSIX shell | Shellcheck in CI. |
| macOS endpoint scripts | POSIX shell | Keep permission/config steps explicit. |
| Mobile config | Manual instructions and QR payloads | Android/iOS operator apps are required before cutover. |
| Compatibility endpoint | Isolated Axum route group | RustDesk-shaped behavior stays outside core domain model. |

## Testing And CI

| Layer | Choice | Notes |
|---|---|---|
| Unit/integration tests | cargo test | Required once scaffold exists. |
| Faster test runner | cargo-nextest | Add once test count justifies it. |
| Formatting | rustfmt | Required in CI. |
| Linting | clippy | Warnings treated as failures for application code. |
| Dependency audit | cargo-audit or cargo-deny | Add before implementation depends on external crates. |
| Container build | Docker/BuildKit | Compose remains first deployment path. |

## Non-Choices

- Do not build a SPA first.
- Do not use Python/Node for the backend without a new ADR.
- Do not link to, copy, or vendor RustDesk AGPL source because OpenDesk is written in Rust.
- Do not introduce Kubernetes for the first production target.
