#!/usr/bin/env python3
import argparse
import json
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


def parse_args():
    parser = argparse.ArgumentParser(
        description="Capture RustDesk deploy requests for local research evidence."
    )
    parser.add_argument("--bind", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18080)
    parser.add_argument(
        "--result",
        default="OK",
        choices=["OK", "NOT_ENABLED", "INVALID_INPUT", "ID_TAKEN", "EMPTY", "TEXT"],
    )
    parser.add_argument(
        "--output",
        default="local/research/manual/deploy-capture.jsonl",
        help="Ignored JSONL evidence path.",
    )
    return parser.parse_args()


def redact_authorization(value):
    if not value:
        return None
    scheme = value.split(" ", 1)[0]
    return f"{scheme} [redacted]"


def redact_body(body):
    try:
        parsed = json.loads(body or "{}")
    except json.JSONDecodeError:
        return {"_unparsed": True}
    for key in ("id", "uuid", "pk"):
        if key in parsed:
            parsed[key] = f"[redacted-{key}]"
    return parsed


def build_handler(result, output_path):
    class DeployCaptureHandler(BaseHTTPRequestHandler):
        def do_POST(self):
            length = int(self.headers.get("content-length", "0"))
            body = self.rfile.read(length).decode("utf-8", "replace")
            record = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "method": self.command,
                "path": self.path,
                "authorization": redact_authorization(self.headers.get("authorization")),
                "content_type": self.headers.get("content-type"),
                "body": redact_body(body),
            }
            with output_path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(record, sort_keys=True) + "\n")

            if result == "EMPTY":
                payload = b""
            elif result == "TEXT":
                payload = b"plain error body"
            else:
                payload = json.dumps({"result": result}).encode("utf-8")

            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, _format, *_args):
            return

    return DeployCaptureHandler


def main():
    args = parse_args()
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.touch(exist_ok=True)
    handler = build_handler(args.result, output_path)
    server = HTTPServer((args.bind, args.port), handler)
    print(f"deploy capture listening on {args.bind}:{args.port}")
    print(f"writing redacted evidence to {output_path}")
    print(f"response result: {args.result}")
    server.serve_forever()


if __name__ == "__main__":
    main()
