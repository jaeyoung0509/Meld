#!/usr/bin/env bash
set -euo pipefail

DOCS_URL="${1:-}"
EXPECTED_MARKER="${2:-Openportio}"
MAX_ATTEMPTS="${3:-10}"
SLEEP_SECONDS="${4:-5}"

if [[ -z "${DOCS_URL}" ]]; then
  echo "[ERROR] usage: $0 <docs-url> [expected-marker] [max-attempts] [sleep-seconds]"
  exit 2
fi

TMP_HEADERS="$(mktemp)"
TMP_BODY="$(mktemp)"

cleanup() {
  rm -f "${TMP_HEADERS}" "${TMP_BODY}"
}
trap cleanup EXIT

echo "[INFO] docs smoke check"
echo "[INFO] url=${DOCS_URL}"
echo "[INFO] expected_marker=${EXPECTED_MARKER}"
echo "[INFO] attempts=${MAX_ATTEMPTS} sleep=${SLEEP_SECONDS}s"

for ((attempt = 1; attempt <= MAX_ATTEMPTS; attempt++)); do
  HTTP_CODE="$(
    curl -sS -L \
      -D "${TMP_HEADERS}" \
      -o "${TMP_BODY}" \
      -w "%{http_code}" \
      "${DOCS_URL}" || true
  )"

  if [[ "${HTTP_CODE}" == "200" ]] && grep -q "${EXPECTED_MARKER}" "${TMP_BODY}"; then
    echo "[OK] docs smoke passed on attempt ${attempt}"
    exit 0
  fi

  echo "[WARN] attempt ${attempt}/${MAX_ATTEMPTS} failed (status=${HTTP_CODE})"
  echo "[WARN] response headers (first 20 lines):"
  sed -n '1,20p' "${TMP_HEADERS}" || true
  echo "[WARN] response body snippet (first 40 lines):"
  sed -n '1,40p' "${TMP_BODY}" || true

  if (( attempt < MAX_ATTEMPTS )); then
    sleep "${SLEEP_SECONDS}"
  fi
done

echo "[ERROR] docs smoke check failed after ${MAX_ATTEMPTS} attempts"
exit 1
