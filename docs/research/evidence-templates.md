# Research Evidence Templates

Use these templates for future validation notes. Site-specific values, screenshots, raw logs, tokens, IDs, keys, hostnames, and IP addresses belong only under ignored `local/research/`.

## Scripted Templates

Use these scripts to create ignored evidence files:

- `scripts/research-client-record.sh`
- `scripts/research-network-probe.sh`
- `scripts/research-windows-client-record.ps1`
- `scripts/research-macos-client-record.sh`
- `scripts/research-mobile-config-record.sh`
- `scripts/research-deploy-capture-server.py`

Both scripts write under `local/research/manual/`, which is intentionally ignored.

## Owner Decision Record

```text
Date:
Owner:
Reviewer:
Decision area:
Decision state: required | equivalent | retired
Reason:
Replacement path:
Validation IDs:
Evidence path:
Expiry/review date if accepted risk:
```

## Client Validation Record

```text
Date:
Tester:
Endpoint OS/version:
RustDesk client version:
Package type:
Installer source:
Signature/checksum result:
Server target:
Commands/scripts used:
Config method:
Config persistence result:
Service/user context result:
Restart result:
Reinstall/upgrade result:
Deploy endpoint result:
Artifacts:
Final status: pass | fail | accepted-exception
Follow-up:
```

## Network Validation Record

```text
Date:
Tester:
Client network context: LAN | WAN | mobile | VPN | other
DNS result:
TCP port result:
UDP result:
Direct connection result:
Relay connection result:
NAT/hairpin finding:
Artifacts:
Final status: pass | fail | accepted-exception
Follow-up:
```

## Mobile Config Record

```text
Date:
Tester:
Mobile OS/version:
RustDesk app version:
Config method: manual | QR
QR payload artifact:
Manual setup result:
Connect-to-endpoint result:
Operator-only decision:
Managed endpoint decision:
Final status: pass | fail | accepted-exception
Follow-up:
```

## Audit Capability Record

```text
Date:
Tester:
Source: OpenDesk audit | RustDesk database | hbbs log | hbbr log | client log
Event type:
Fields available:
Fields missing:
Can prove session start:
Can prove session end:
Can prove actor:
Can prove endpoint:
Retention behavior:
Artifacts:
Decision: launch-intent | ingest | deeper integration | unsupported
```
