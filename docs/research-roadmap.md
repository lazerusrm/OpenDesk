# Research Roadmap

This document indexes the remaining research required to turn the plan into a working full replacement. Each item must produce evidence before it can be closed.

## Evidence Standard

Every research item needs:

- Research owner.
- Date.
- Source links or local evidence path.
- Tested RustDesk client/server versions where applicable.
- Tested operating systems where applicable.
- Decision: `supported`, `unsupported`, `requires workaround`, or `requires implementation`.
- Follow-up tasks or validation IDs.

Research notes with site-specific values must live under ignored `local/`.

## Research Tracks

- [Client And Deployment Research](research/client-deployment.md)
- [Operations And Security Research](research/operations-security.md)
- [Current Research Findings](research-findings.md)
- [Dev Validation Environment](dev-validation.md)

## Required Items

| ID | Topic | Track | Blocking Question |
|---|---|---|---|
| R-001 | Client configuration behavior by OS/version | Client And Deployment | Which official client config mechanisms work reliably per OS/package/version? |
| R-002 | Official client deployment mechanics | Client And Deployment | How do we install, configure, upgrade, and verify official clients without Pro custom clients? |
| R-003 | Current RustDesk Pro usage inventory | Operations And Security | Which Pro features are actually used in production today? |
| R-004 | Address book and password model | Operations And Security | Does production rely on managed/passwordless address-book behavior? |
| R-005 | Access control reality | Operations And Security | What enforcement is required beyond OpenDesk dashboard RBAC? |
| R-006 | Deployment endpoint compatibility | Client And Deployment | Can stock official clients use an OpenDesk implementation of the RustDesk-shaped deploy endpoint? |
| R-007 | Session and audit log sources | Operations And Security | What audit visibility can OpenDesk provide without modifying RustDesk? |
| R-008 | Mobile workflow | Client And Deployment | What does full replacement require for iOS/Android operators or endpoints? |
| R-009 | Relay scaling, NAT, LAN, and DNS behavior | Operations And Security | What network behavior must OpenDesk document and validate? |
| R-010 | Legal and license posture | Operations And Security | What license boundary keeps OpenDesk clean-room, and what changes if we fork or link RustDesk code? |

## Cutover Rule

Every item above must have accepted evidence and a decision before production cutover. Any `unknown` result blocks cutover unless the owner explicitly retires the related workflow.
