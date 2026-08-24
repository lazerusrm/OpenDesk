#!/usr/bin/env bash
# Fetch official RustDesk OSS hbbs/hbbr. They are AGPL and are not OpenDesk.
# This script downloads upstream binaries or the official container image.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

RELEASES_API="https://api.github.com/repos/rustdesk/rustdesk-server/releases/latest"
IMAGE="docker.io/rustdesk/rustdesk-server:latest"
DEST_DIR="${OPENDESK_RUSTDESK_SERVER_DIR:-$repo_root/data/rustdesk-server}"

want="ask"
start="no"
method="auto"
print_asset="no"
select_json=""

usage() {
  cat <<'EOF'
Usage: scripts/fetch-rustdesk-server.sh [options]

Fetch official RustDesk OSS hbbs/hbbr (AGPL, not part of OpenDesk).
On a TTY this asks first. Non-interactive runs skip unless --yes or
OPENDESK_FETCH_RUSTDESK_SERVER=1.

  --yes              fetch latest without prompting
  --no               skip fetch
  --start            after a Docker fetch, start Compose profile rustdesk-server
  --method docker    pull rustdesk/rustdesk-server:latest
  --method github    download the latest GitHub zip and verify sha256
  --print-asset      print the zip name for this machine and exit
  --select-json FILE resolve zip URL/digest from a saved GitHub release JSON
  -h, --help         show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes) want="yes" ;;
    --no) want="no" ;;
    --start) start="yes" ;;
    --method)
      method="${2:-}"
      shift
      ;;
    --print-asset) print_asset="yes" ;;
    --select-json)
      select_json="${2:-}"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

arch_name() {
  case "$(uname -m)" in
    x86_64 | amd64) printf '%s\n' "amd64" ;;
    aarch64 | arm64) printf '%s\n' "arm64v8" ;;
    armv7l | armv7) printf '%s\n' "armv7" ;;
    i386 | i686) printf '%s\n' "i386" ;;
    *)
      echo "unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

asset_zip() {
  printf 'rustdesk-server-linux-%s.zip\n' "$(arch_name)"
}

if [[ "$print_asset" == "yes" ]]; then
  asset_zip
  exit 0
fi

select_from_json() {
  local json_path="$1"
  local zip_name
  zip_name="$(asset_zip)"
  python3 - "$json_path" "$zip_name" <<'PY'
import json
import sys

path, wanted = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)
for asset in payload.get("assets") or []:
    if asset.get("name") == wanted:
        digest = str(asset.get("digest") or "")
        if digest.startswith("sha256:"):
            digest = digest.split(":", 1)[1]
        url = asset.get("browser_download_url") or ""
        if not url.startswith("https://github.com/rustdesk/rustdesk-server/"):
            sys.exit("download URL is not the official rustdesk-server release host")
        print(payload.get("tag_name") or "")
        print(url)
        print(digest)
        sys.exit(0)
sys.exit(f"release has no asset named {wanted}")
PY
}

if [[ -n "$select_json" ]]; then
  select_from_json "$select_json"
  exit 0
fi

env_want="${OPENDESK_FETCH_RUSTDESK_SERVER:-}"
case "${env_want,,}" in
  1 | true | yes | y) want="yes" ;;
  0 | false | no | n) [[ "$want" == "ask" ]] && want="no" ;;
esac

if [[ "$want" == "ask" ]]; then
  if [[ -t 0 && -t 1 ]]; then
    echo "OpenDesk can fetch official RustDesk OSS hbbs/hbbr."
    echo "They are AGPL, unmodified upstream programs, not OpenDesk source."
    read -r -p "Fetch the latest official image or binaries now? [y/N] " reply
    case "${reply,,}" in
      y | yes) want="yes" ;;
      *) want="no" ;;
    esac
  else
    echo "no TTY: skipping hbbs/hbbr fetch (pass --yes to fetch)"
    want="no"
  fi
fi

if [[ "$want" != "yes" ]]; then
  echo "skipped official hbbs/hbbr fetch"
  echo "OpenDesk-only: docker compose up --build"
  echo "OpenDesk plus official hbbs/hbbr:"
  echo "  bash scripts/fetch-rustdesk-server.sh --yes --start"
  exit 0
fi

have_docker() {
  command -v docker >/dev/null 2>&1
}

compose_cmd() {
  if docker compose version >/dev/null 2>&1; then
    echo "docker compose"
  elif command -v docker-compose >/dev/null 2>&1; then
    echo "docker-compose"
  else
    return 1
  fi
}

fetch_docker() {
  echo "pulling $IMAGE"
  docker pull "$IMAGE"
  echo "official hbbs/hbbr image is ready (AGPL, not OpenDesk)."
  echo "start OpenDesk plus transport with one command:"
  echo "  docker compose --profile rustdesk-server up --build"
  echo "then copy the public key (after hbbs has started once):"
  echo "  docker compose --profile rustdesk-server exec hbbs cat /root/id_ed25519.pub"
  echo "open http://127.0.0.1:8080/setup as admin."
  echo "lab fields: Public URL and API = http://127.0.0.1:8080 ; ID and relay = 127.0.0.1"
}

fetch_github() {
  local tmp json zip_name tag url digest archive
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/opendesk-hbbs.XXXXXX")"
  json="$tmp/latest.json"
  echo "querying official rustdesk-server latest release"
  curl -fsSL -A "OpenDesk-hbbs-fetch/1" -o "$json" "$RELEASES_API"
  zip_name="$(asset_zip)"
  mapfile -t meta < <(select_from_json "$json")
  tag="${meta[0]}"
  url="${meta[1]}"
  digest="${meta[2]}"
  if [[ -z "$url" || -z "$digest" ]]; then
    echo "could not resolve $zip_name from $tag" >&2
    exit 1
  fi
  archive="$tmp/$zip_name"
  echo "downloading $zip_name ($tag)"
  curl -fsSL -A "OpenDesk-hbbs-fetch/1" -o "$archive" "$url"
  echo "$digest  $archive" | sha256sum -c -
  mkdir -p "$DEST_DIR"
  unzip -o -q "$archive" -d "$tmp/extract"
  while IFS= read -r -d '' binary; do
    install -m 0755 "$binary" "$DEST_DIR/$(basename "$binary")"
  done < <(find "$tmp/extract" -type f \( -name hbbs -o -name hbbr -o -name rustdesk-utils \) -print0)
  if [[ ! -x "$DEST_DIR/hbbs" || ! -x "$DEST_DIR/hbbr" ]]; then
    echo "zip did not contain hbbs and hbbr" >&2
    exit 1
  fi
  rm -rf "$tmp"
  echo "installed $tag to $DEST_DIR"
  echo "from that directory start relay, then id server:"
  echo "  ./hbbr"
  echo "  ./hbbs -r 127.0.0.1:21117"
  echo "after hbbs starts, paste $DEST_DIR/id_ed25519.pub into /setup."
  echo "prefer Docker when you can: bash scripts/fetch-rustdesk-server.sh --yes --method docker"
}

resolved="$method"
if [[ "$resolved" == "auto" ]]; then
  if have_docker; then
    resolved="docker"
  else
    resolved="github"
  fi
fi

case "$resolved" in
  docker)
    if ! have_docker; then
      echo "docker is not installed" >&2
      exit 1
    fi
    fetch_docker
    if [[ "$start" == "yes" ]]; then
      cmd="$(compose_cmd)" || {
        echo "docker compose is not available" >&2
        exit 1
      }
      $cmd --profile rustdesk-server up --build -d
    elif [[ -t 0 && -t 1 ]]; then
      read -r -p "Start official hbbs/hbbr with Compose now? [y/N] " reply
      case "${reply,,}" in
        y | yes)
          cmd="$(compose_cmd)" || {
            echo "docker compose is not available" >&2
            exit 1
          }
          $cmd --profile rustdesk-server up --build -d
          ;;
      esac
    fi
    ;;
  github)
    command -v curl >/dev/null 2>&1 || {
      echo "curl is required" >&2
      exit 1
    }
    command -v unzip >/dev/null 2>&1 || {
      echo "unzip is required" >&2
      exit 1
    }
    fetch_github
    ;;
  *)
    echo "unknown method: $resolved (use docker or github)" >&2
    exit 2
    ;;
esac
