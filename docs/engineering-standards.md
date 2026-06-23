# Engineering Standards

OpenDesk should be built as a clean, canonical system. Fast iterative development often introduces compatibility layers, vague names, oversized files, and "temporary" shims that become permanent. This document sets the default standard against that drift.

## Canonical Contracts

Every concept should have one canonical name and one canonical data shape from producer to consumer.

Rules:

- Define canonical domain names in the data model before implementation.
- Use the same field names in database schema, API DTOs, service code, UI bindings, validation docs, and generated artifacts unless there is a documented reason not to.
- Do not translate between multiple internal synonyms such as `device`, `host`, `machine`, and `endpoint` casually.
- If an external system uses a different name, isolate that mapping at the boundary.
- Do not let compatibility naming leak into core domain code.

Canonical domain vocabulary:

| Concept | Canonical Name | Avoid |
|---|---|---|
| Managed remote system | `device` | `host`, `machine`, `endpoint` in core domain |
| RustDesk numeric/string ID | `rustdesk_id` | `id`, `remote_id`, `desk_id` |
| Internal stable ID | `device_uuid` | overloaded `id` |
| Physical/customer grouping | `site` | `location`, `customer_site` unless modeled separately |
| Deployment credential for enrollment | `enrollment_token` | `deploy_token`, `register_token` |
| RustDesk ID server | `id_server` | `rendezvous` in UI/API, except upstream docs |
| RustDesk relay server | `relay_server` | `hbbr_host` outside low-level config |
| OpenDesk admin service | `opendesk` | `control_plane` in runtime names |
| Generated install flow | `deployment_artifact` | `installer_blob`, `download_thing` |

## Anti-Shim Policy

Shims, adapters, and compatibility paths are allowed only at explicit boundaries.

Allowed:

- Boundary adapters for RustDesk-specific config names.
- Compatibility endpoint handlers isolated from OpenDesk-native APIs.
- Migration code with a planned removal point.
- Test fixtures that intentionally model legacy/external behavior.

Rejected:

- Internal aliases that exist only because a prior iteration used the wrong name.
- "Support both old and new fields" before there is a shipped old field.
- Fallback behavior that hides broken producer/consumer contracts.
- Compatibility layers between two OpenDesk modules that should share one canonical contract.
- Legacy flags before there is actual legacy behavior.
- Dead code kept because a contributor was uncertain.

## Tool-Origin Attribution Policy

Public code, docs, comments, commit messages, generated artifacts, and UI text must read as human-authored project work. Do not mention assistant/tool/vendor names, tool-specific folders, or tool-specific instruction files in public project content.

If attribution-like text is unavoidable in private local notes, use a normal human maintainer name and keep it under ignored local paths. Prefer no attribution at all.

Any compatibility behavior must document:

- External system or released version requiring it.
- Owner.
- Tests.
- Removal condition, if temporary.

## Naming Discipline

Names should be boring, precise, and stable.

Rules:

- Prefer domain names over implementation names.
- Avoid vague nouns: `data`, `payload`, `info`, `thing`, `manager`, `helper`, `util`.
- Avoid ambiguous IDs. Use `device_uuid`, `rustdesk_id`, `user_uuid`, `site_uuid`.
- Avoid boolean names that hide policy. Prefer `requires_approval` over `enabled`.
- Avoid suffix creep such as `V2`, `New`, `Old`, `Final`, `Temp`.
- Do not abbreviate domain concepts unless the abbreviation is canonical: `RBAC`, `OIDC`, `API`.
- Function names should describe the domain action: `enroll_device`, `archive_device`, `render_deployment_script`.

## File and Module Size Limits

Soft limits exist to prevent monoliths. Exceeding them requires a short note in code review or an ADR if permanent.

| File Type | Soft Limit | Action When Exceeded |
|---|---:|---|
| Go/Rust/Python source file | 400 lines | Split by domain responsibility. |
| TypeScript/JavaScript source file | 350 lines | Split UI components/services. |
| HTML/template file | 300 lines | Split partials/components. |
| CSS file | 400 lines | Split by layout/component/theme. |
| Markdown planning doc | 250 lines | Split into focused docs. |
| Test file | 500 lines | Split by behavior or fixture domain. |

Hard warning signs:

- More than one domain aggregate in a file.
- Mixed HTTP handlers, database queries, and template rendering in one file.
- Large switch/case blocks for behavior that belongs in typed policies.
- A file named `utils`, `helpers`, `common`, or `misc`.
- Repeated string constants for field names instead of shared typed definitions.

## Producer-to-Consumer Integrity

Data should be validated at the producer boundary and consumed without guessing.

Rules:

- API responses should have explicit schemas.
- Generated scripts should be rendered from typed config objects.
- UI should not infer missing backend fields.
- Backend should reject unknown or conflicting fields for core APIs.
- If a field is optional, document why and what the consumer must do.
- No silent fallback to defaults for security-sensitive values.
- No hidden compatibility transforms in generic middleware.

## Review Checklist

Before merging implementation work, check:

- Does this introduce a second name for an existing concept?
- Does this create a shim between OpenDesk modules?
- Is compatibility isolated at an external boundary?
- Does every new field have one producer and known consumers?
- Are file sizes below soft limits?
- Are functions named after domain actions?
- Are security-sensitive defaults explicit?
- Are tests validating the canonical contract rather than implementation quirks?
