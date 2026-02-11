#!/usr/bin/env python3
"""Generate a dev-only HS256 JWT for local Meld auth testing."""

from __future__ import annotations

import argparse
import base64
import hashlib
import hmac
import json
import sys
import time


def b64url(value: bytes) -> str:
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a development JWT (HS256) for local testing."
    )
    parser.add_argument("--secret", required=True, help="HMAC secret")
    parser.add_argument("--issuer", required=True, help="JWT issuer (iss)")
    parser.add_argument("--audience", required=True, help="JWT audience (aud)")
    parser.add_argument(
        "--subject",
        default="dev-user",
        help="JWT subject (sub), default: dev-user",
    )
    parser.add_argument(
        "--exp-seconds",
        type=int,
        default=3600,
        help="Token expiration from now in seconds, default: 3600",
    )
    args = parser.parse_args()
    if args.exp_seconds <= 0:
        parser.error("--exp-seconds must be greater than 0")
    return args


def main() -> int:
    args = parse_args()
    now = int(time.time())

    header = {"alg": "HS256", "typ": "JWT"}
    payload = {
        "sub": args.subject,
        "iss": args.issuer,
        "aud": args.audience,
        "iat": now,
        "exp": now + args.exp_seconds,
    }

    header_part = b64url(json.dumps(header, separators=(",", ":"), sort_keys=True).encode())
    payload_part = b64url(json.dumps(payload, separators=(",", ":"), sort_keys=True).encode())

    signing_input = f"{header_part}.{payload_part}".encode("ascii")
    signature = hmac.new(
        args.secret.encode("utf-8"),
        signing_input,
        hashlib.sha256,
    ).digest()
    token = f"{header_part}.{payload_part}.{b64url(signature)}"

    sys.stdout.write(token)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
