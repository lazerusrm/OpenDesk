# Research Roadmap

This document indexes the remaining research required to turn the plan into a working full replacement. Each item must produce evidence before it can be closed. The authorized read-only baseline establishes populated Pro data structures and observed official runtime components, but does not establish production usage, parity, or signoff.

## Evidence Standard

Every research item needs:

- Research owner.
- Date.
- Source links or local evidence path.
- Tested RustDesk client/server versions where applicable.
- Tested operating systems where applicable.
- Decision: `supported`, `unsupported`, `requires workaround`, or `requires implementation`.
- Follow-up tasks or validation IDs.

Research notes with site-specific values must live under ignored `local/`. Public documents must not include private topology, counts, addresses, hostnames, keys, or credentials.

## Research Tracks

- [Client And Deployment Research](research/client-deployment.md)
- [Operations And Security Research](research/operations-security.md)
- [Owner Decision Worksheet](research/owner-decisions.md)
- [Client Validation Procedures](research/client-validation-procedures.md)
- [Research Evidence Templates](research/evidence-templates.md)
- [Research Status](research-status.md)
- [Research Closure Packets](research-closure-packets.md)
- [Current Research Findings](research-findings.md)
- [Dev Validation Environment](dev-validation.md)
- [Validation Lab](validation-lab.md)
- [Implementation Handoff](implementation-handoff.md)

## Required Items

| ID | Topic | Established baseline | Still-required decision/evidence |
|---|---|---|---|
| R-001 | Client configuration behavior by OS/version | Linux package/config evidence and official documentation/source review exist. | Execute Windows, Linux, macOS, Android, and iOS validation with released client versions; classify each config path. |
| R-002 | Official client deployment mechanics | Linux install/service/restart/reinstall behavior is evidenced; generated official-client flow is the current direction. | Validate Windows/macOS install, service or user config, upgrade persistence, and mobile operator setup. |
| R-003 | Current RustDesk Pro usage inventory | Read-only discovery found populated peers, users, groups/mappings, address books/linked peers, strategies, custom-client, session, console, and audit structures. | Owner interviews/exports must classify weekly use, replacement path, validation IDs, evidence, and blocker for every capability. |
| R-004 | Address book and password model | Address-book entries contain hashed secret material; plaintext storage is prohibited and ADR-008 defines a vault path if required. | Owner decides whether managed/passwordless access is required, equivalent, or retired; validate selected path. |
| R-005 | Access control reality | Owner selected dashboard/API RBAC only; this is not RustDesk session authorization. | Validate access-boundary wording and permissions. Keep endpoint/network/client enforcement as future research unless separately proposed; resolve password-sharing and session-audit decisions. |
| R-006 | Deployment endpoint compatibility | Linux deploy request/response behavior is validated against a controlled endpoint. | Validate Windows/macOS clients and isolated adapter behavior before claiming compatibility. |
| R-007 | Session and audit log sources | Read-only audit tables/logs are populated and show useful connection, console, relay, rendezvous, and recent console visibility. | Owner chooses audit tier; validate ingestion/labels if selected. Do not equate launch intent with session proof. |
| R-008 | Mobile workflow | Official documentation supports Android manual/QR configuration and identifies iOS as operator-only for this scope. | Execute Android and iOS operator workflows with OpenDesk-generated instructions/configuration. |
| R-009 | Relay scaling, NAT, LAN, and DNS behavior | Observed official `hbbs`/`hbbr` and admin-web runtime plus controlled probes establish a baseline without publishing topology. | Execute real-client WAN, mobile-network, NAT, direct-vs-relay, and failure tests on required desktop platforms. |
| R-010 | Legal and license posture | ADR records the clean-room control-plane boundary and fork/link/vendor rules. | None unless fork/vendor work is proposed. |

## Cutover Rule

Every item above must have accepted evidence and a decision before production cutover. Any `unknown` result blocks cutover unless the owner explicitly retires the related workflow. Current research remains partial except R-010; no roadmap row is a signoff.
