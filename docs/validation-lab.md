# Validation Lab

This document defines the test environments required to close the remaining research blockers. Private VM names, IPs, credentials, screenshots, and raw logs belong under ignored `local/research/`.

## Required Lab Targets

| Target | Purpose | Required For |
|---|---|---|
| Windows 11 VM or spare endpoint | Installer, service context, config persistence, deploy endpoint, operator workflow. | R-001, R-002, R-006, R-009 |
| Windows portable test context | Filename config, duplicate filename behavior, portable elevation limits. | R-001, R-002 |
| macOS test endpoint | DMG install, permissions, user/root config persistence, operator workflow. | R-001, R-002, R-006, R-009 |
| Android phone | Official RustDesk app operator config and connection test. | R-008, D-011 |
| iPhone or iPad | Official RustDesk app operator config and connection test. | R-008, D-012 |
| LAN test endpoint | Direct/local behavior and split-DNS/hairpin check. | R-009 |
| WAN or mobile-hotspot endpoint | External DNS/NAT/direct-vs-relay behavior. | R-009 |

## Lab Rules

- Use disposable endpoints or snapshots.
- Use test-only RustDesk IDs, keys, tokens, and passwords.
- Do not copy production private keys, license material, production database files, or unattended passwords into lab machines.
- Record raw evidence under ignored `local/research/manual/`.
- Promote only sanitized conclusions to public docs.
- Run official released RustDesk clients, not locally built clients, unless a test explicitly covers fork behavior.

## Minimum Evidence Per Target

| Evidence | Windows | macOS | Android | iOS | Network |
|---|---:|---:|---:|---:|---:|
| Client version | Required | Required | Required | Required | Optional |
| Package/source | Required | Required | Required | Required | Optional |
| Config method | Required | Required | Required | Required | Optional |
| Config persistence | Required | Required | Manual check | Manual check | Optional |
| Deploy endpoint capture | Required | Required if supported | Not required | Not required | Optional |
| Connection to test endpoint | Required | Required | Required | Required | Required |
| Direct vs relay result | Required | Required | Required | Required | Required |
| Screenshot/log artifact | Required | Required | Required | Required | Required |

## Closure Mapping

- R-001 closes when Windows installer, Windows portable, Linux package, macOS, Android, and iOS config behaviors have accepted evidence or explicit retirement.
- R-002 closes when Windows, Linux, and macOS install/config/restart/reinstall or upgrade persistence are accepted.
- R-006 closes when deploy endpoint behavior is validated for required desktop clients and adapter tests exist.
- R-008 closes when Android and iOS operator workflows pass with OpenDesk-generated config/instructions.
- R-009 closes when LAN, WAN, mobile-network, DNS, NAT/hairpin, and direct-vs-relay behavior are accepted.
