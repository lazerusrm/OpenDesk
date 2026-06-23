# Dev Validation Environment

OpenDesk validation should use disposable dev infrastructure before any production cutover test.

## Current Dev Target

A dev LXC has been provisioned and left running. Private VMID, IP address, and host details live only under ignored `local/research/`.

Use this target for:

- Prototyping the OpenDesk API and web UI.
- Testing generated Linux install/config scripts.
- Testing RustDesk-shaped deploy endpoint compatibility.
- Testing failure behavior without touching production services.
- Recording package/config persistence behavior before production rollout.

Do not use this target for:

- Production RustDesk database copies.
- Production private keys.
- Production license material.
- Production client IDs or unattended passwords.
- Long-lived privileged tokens.

## Storage Rule

Use storage with enough free capacity for package caches, build artifacts, logs, and snapshots. If a storage pool is already high-utilization, do not place dev validation containers there unless explicitly approved.

## Evidence Rule

Raw dev environment details belong under ignored `local/research/`. Public docs may describe the validation method, but not private addresses, hostnames, credentials, keys, or production topology.
