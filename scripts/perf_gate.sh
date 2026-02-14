#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MELD_PERF_BOOT_SERVER="${MELD_PERF_BOOT_SERVER:-true}"
MELD_PERF_WAIT_SECONDS="${MELD_PERF_WAIT_SECONDS:-120}"
MELD_PERF_BASE_URL="${MELD_PERF_BASE_URL:-http://127.0.0.1:3000}"
MELD_PERF_GRPC_ADDR="${MELD_PERF_GRPC_ADDR:-127.0.0.1:3000}"

MELD_PERF_REST_PATH="${MELD_PERF_REST_PATH:-/health}"
MELD_PERF_REST_VUS="${MELD_PERF_REST_VUS:-20}"
MELD_PERF_REST_DURATION="${MELD_PERF_REST_DURATION:-12s}"
MELD_PERF_REST_P95_MS="${MELD_PERF_REST_P95_MS:-120}"
MELD_PERF_REST_ERR_RATE="${MELD_PERF_REST_ERR_RATE:-0.01}"

MELD_PERF_GRPC_REQUESTS="${MELD_PERF_GRPC_REQUESTS:-500}"
MELD_PERF_GRPC_CONCURRENCY="${MELD_PERF_GRPC_CONCURRENCY:-25}"
MELD_PERF_GRPC_P95_MS="${MELD_PERF_GRPC_P95_MS:-120}"
MELD_PERF_GRPC_ERR_RATE="${MELD_PERF_GRPC_ERR_RATE:-0.01}"

PERF_DIR="target/perf"
SERVER_PID=""
SERVER_LOG=""

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[FAIL] required tool '$tool' is not installed"
    exit 1
  fi
}

is_truthy() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$SERVER_LOG" && -f "$SERVER_LOG" ]]; then
    rm -f "$SERVER_LOG"
  fi
}

wait_for_server() {
  local base_url="$1"
  local wait_seconds="$2"
  local attempts=$((wait_seconds * 4))

  for _ in $(seq 1 "$attempts"); do
    if curl -fsS "$base_url/health" >/dev/null 2>&1; then
      return 0
    fi
    if [[ -n "$SERVER_PID" ]] && ! kill -0 "$SERVER_PID" >/dev/null 2>&1; then
      return 1
    fi
    sleep 0.25
  done

  return 1
}

start_server_if_needed() {
  if ! is_truthy "$MELD_PERF_BOOT_SERVER"; then
    return 0
  fi

  SERVER_LOG="$(mktemp)"
  cargo run -p meld-server --bin meld-server >"$SERVER_LOG" 2>&1 &
  SERVER_PID="$!"

  if wait_for_server "$MELD_PERF_BASE_URL" "$MELD_PERF_WAIT_SECONDS"; then
    echo "[OK] meld-server booted for perf gate"
  else
    echo "[FAIL] server boot failed within ${MELD_PERF_WAIT_SECONDS}s"
    if [[ -f "$SERVER_LOG" ]]; then
      echo "[INFO] server log tail:"
      tail -n 80 "$SERVER_LOG" || true
    fi
    exit 1
  fi
}

run_rest_k6() {
  echo "[REST] running k6 scenario"

  K6_REST_BASE_URL="$MELD_PERF_BASE_URL" \
  K6_REST_PATH="$MELD_PERF_REST_PATH" \
  K6_REST_VUS="$MELD_PERF_REST_VUS" \
  K6_REST_DURATION="$MELD_PERF_REST_DURATION" \
  K6_REST_P95_MS="$MELD_PERF_REST_P95_MS" \
  K6_REST_ERR_RATE="$MELD_PERF_REST_ERR_RATE" \
  k6 run \
    --summary-export "$PERF_DIR/rest-k6-summary.json" \
    perf/rest_smoke.js
}

run_grpc_ghz() {
  echo "[gRPC] running ghz scenario"

  ghz \
    --insecure \
    --proto crates/meld-rpc/proto/service.proto \
    --call meld.v1.Greeter.SayHello \
    --data '{"name":"perf"}' \
    --total "$MELD_PERF_GRPC_REQUESTS" \
    --concurrency "$MELD_PERF_GRPC_CONCURRENCY" \
    --format json \
    --output "$PERF_DIR/grpc-ghz-summary.json" \
    "$MELD_PERF_GRPC_ADDR"
}

analyze_grpc_result() {
  python3 - "$PERF_DIR/grpc-ghz-summary.json" "$MELD_PERF_GRPC_P95_MS" "$MELD_PERF_GRPC_ERR_RATE" <<'PY'
import json
import re
import sys

summary_path = sys.argv[1]
expected_p95_ms = float(sys.argv[2])
expected_err_rate = float(sys.argv[3])

with open(summary_path, "r", encoding="utf-8") as fh:
    payload = json.load(fh)

def duration_to_ms(value: str) -> float:
    value = value.strip()
    m = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)(ns|us|µs|ms|s|m|h)", value)
    if not m:
        raise ValueError(f"unsupported duration format: {value}")
    amount = float(m.group(1))
    unit = m.group(2)
    if unit == "ns":
        return amount / 1_000_000
    if unit in ("us", "µs"):
        return amount / 1_000
    if unit == "ms":
        return amount
    if unit == "s":
        return amount * 1_000
    if unit == "m":
        return amount * 60_000
    if unit == "h":
        return amount * 3_600_000
    raise ValueError(f"unsupported unit: {unit}")

count = int(payload.get("count", 0))
if count <= 0:
    print("[FAIL] invalid ghz count in summary", file=sys.stderr)
    sys.exit(1)

status_distribution = payload.get("statusCodeDistribution") or {}
ok_count = int(status_distribution.get("OK", 0))
error_count = max(count - ok_count, 0)
error_rate = error_count / count

latency_distribution = payload.get("latencyDistribution") or []
p95_entry = next((item for item in latency_distribution if int(item.get("percentage", -1)) == 95), None)
if p95_entry is None:
    print("[FAIL] missing p95 entry in ghz latencyDistribution", file=sys.stderr)
    sys.exit(1)

p95_ms = duration_to_ms(str(p95_entry.get("latency", "")))

print(f"[gRPC] count={count} ok={ok_count} error_rate={error_rate:.5f} p95_ms={p95_ms:.2f}")

failed = False
if error_rate > expected_err_rate:
    print(
        f"[FAIL] gRPC error_rate {error_rate:.5f} exceeded threshold {expected_err_rate:.5f}",
        file=sys.stderr,
    )
    failed = True

if p95_ms > expected_p95_ms:
    print(
        f"[FAIL] gRPC p95 {p95_ms:.2f}ms exceeded threshold {expected_p95_ms:.2f}ms",
        file=sys.stderr,
    )
    failed = True

with open("target/perf/grpc-evaluation.txt", "w", encoding="utf-8") as out:
    out.write(f"count={count}\n")
    out.write(f"ok={ok_count}\n")
    out.write(f"error_rate={error_rate:.5f}\n")
    out.write(f"p95_ms={p95_ms:.2f}\n")
    out.write(f"threshold_error_rate={expected_err_rate:.5f}\n")
    out.write(f"threshold_p95_ms={expected_p95_ms:.2f}\n")

if failed:
    sys.exit(1)
PY
}

main() {
  trap cleanup EXIT

  require_tool curl
  require_tool python3
  require_tool k6
  require_tool ghz

  mkdir -p "$PERF_DIR"

  start_server_if_needed

  run_rest_k6
  run_grpc_ghz
  analyze_grpc_result

  cat > "$PERF_DIR/summary.txt" <<SUMMARY
REST summary: $PERF_DIR/rest-k6-summary.json
gRPC summary: $PERF_DIR/grpc-ghz-summary.json
gRPC evaluation: $PERF_DIR/grpc-evaluation.txt
Thresholds:
  REST p95 <= ${MELD_PERF_REST_P95_MS}ms
  REST error rate <= ${MELD_PERF_REST_ERR_RATE}
  gRPC p95 <= ${MELD_PERF_GRPC_P95_MS}ms
  gRPC error rate <= ${MELD_PERF_GRPC_ERR_RATE}
SUMMARY

  echo "[OK] perf gate passed"
  cat "$PERF_DIR/summary.txt"
}

main "$@"
