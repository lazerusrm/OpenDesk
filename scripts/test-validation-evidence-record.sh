#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
recorder="$script_dir/validation-evidence-record.sh"

base_args=(--case smoke --platform linux --client-version 1.2.3 --package deb)
expect_rejected() {
  local label="$1"
  shift
  if "$recorder" "${base_args[@]}" "$@" >/dev/null 2>&1; then
    printf 'expected rejection: %s\n' "$label" >&2
    exit 1
  fi
}

expect_rejected "case whitespace" --case 'bad case'
expect_rejected "case topology" --case 'test-host'
expect_rejected "case secret marker" --case 'test-token'
expect_rejected "client URL" --client-version 'https://example.test'
expect_rejected "client IP-like version" --client-version '1.2.3.4'
expect_rejected "client whitespace" --client-version '1.2.3 beta'
expect_rejected "package free form" --package 'custom-package'
expect_rejected "status free form" --access 'dashboard allowed'

printf 'validation-evidence-record rejection tests passed\n'
