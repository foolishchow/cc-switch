#!/usr/bin/env bash
# REST 控制面冒烟脚本（Phase 1）。
# 前置：app 已以 `rest_api` feature 构建 + 启动，且 settings.restApi.enabled=true。
set -euo pipefail

SETTINGS="${CC_SWITCH_SETTINGS:-$HOME/.cc-switch/settings.json}"
TOKEN=$(jq -r '.restApi.token // empty' "$SETTINGS")
HOST=$(jq -r '.restApi.listenAddress // "127.0.0.1"' "$SETTINGS")
PORT=$(jq -r '.restApi.listenPort // 8787' "$SETTINGS")
BASE="http://${HOST}:${PORT}/control/v1"

if [ -z "$TOKEN" ]; then
  echo "✗ token 为空（未启用 REST？）" >&2; exit 1
fi
AUTH=(-H "Authorization: Bearer ${TOKEN}")

echo "== A-003 未授权 401 =="
curl -sS -o /dev/null -w "%{http_code}\n" "${BASE}/providers?app=claude"

echo "== A-004 GET /providers =="
curl -sS "${AUTH[@]}" "${BASE}/providers?app=claude" | jq -r 'keys | length' 

echo "== A-005 GET /providers/current =="
curl -sS "${AUTH[@]}" "${BASE}/providers/current?app=claude" | jq -r '.current'

echo "== A-024 POST /proxy/restart（幂等） =="
curl -sS "${AUTH[@]}" -X POST "${BASE}/proxy/restart" >/dev/null
curl -sS "${AUTH[@]}" "${BASE}/proxy/status" | jq -r '.running'

echo "== switch 后 current 应变化（A-006） =="
echo "（手动：curl -X POST ${BASE}/providers/switch -d '{\"app\":\"claude\",\"provider_id\":\"<id>\"}'）"
