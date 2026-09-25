#!/usr/bin/env bash
# Black-box checks against a running backend. Usage: scripts/smoke-test.sh [base_url]
set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
fail() { echo "FAIL: $*" >&2; exit 1; }

echo "waiting for $BASE_URL/healthz"
for _ in $(seq 1 60); do
  curl -fsS "$BASE_URL/healthz" >/dev/null 2>&1 && break
  sleep 1
done

body=$(curl -sS -D /tmp/smoke-headers -w '\n%{http_code}' "$BASE_URL/healthz")
code=${body##*$'\n'}; body=${body%$'\n'*}
[[ "$code" == 200 ]] || fail "/healthz returned $code: $body"
[[ "$body" == *'"status":"ok"'* && "$body" == *'"database":"ok"'* && "$body" == *'"redis":"ok"'* ]] \
  || fail "/healthz body not healthy: $body"
grep -qi '^x-request-id:' /tmp/smoke-headers || fail "missing x-request-id header"
echo "ok   /healthz -> $body"

body=$(curl -sS -w '\n%{http_code}' "$BASE_URL/v1/does-not-exist")
code=${body##*$'\n'}; body=${body%$'\n'*}
[[ "$code" == 404 && "$body" == *'"code":"NOT_FOUND"'* ]] || fail "unknown route: $code $body"
echo "ok   unknown route -> 404 $body"

echo "smoke test passed"
