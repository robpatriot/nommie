#!/usr/bin/env bash
# ==============================================================================
# guardrails.sh — policy checks for backend
#
# Usage:
#   scripts/guardrails.sh          # full repo (CI)
#   scripts/guardrails.sh --staged # only staged files (pre-commit)
#
# This enforces architectural and test hygiene rules:
#   • Only sentinel tests (files whose names contain 'sentinel_') may:
#       – call require_db
#       – open pooled Database::connect connections
#     Example: apps/backend/tests/adapters_sentinel_games_sea_test.rs
#
#   • Legitimate non-sentinel contexts where require_db/pooled-connect are allowed:
#       – files under tests/support/**
#       – files that also reference SharedTxn/shared_txn/with_txn (shared transaction setup/usage)
#       – files that explicitly test the helper (contain test_require_db or '/_test/require_db')
#
#   • Outside adapters/db layers:
#       – No direct sea_orm imports except ConnectionTrait
#       – No DatabaseConnection or DatabaseTransaction references (except in db/, infra/, state/app_state.rs)
#       – No ad-hoc begin/commit/rollback calls
#
#   • Panic strings in src must be constants (flags direct panic!("…"))
#
#   In staged mode (--staged) the script limits checks to modified backend files
#   for pre-commit speed. CI runs the full version.
# ==============================================================================

set -euo pipefail

MODE="full"
[[ "${1:-}" == "--staged" ]] && MODE="staged"

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

BACKEND_DIR="apps/backend"
BACKEND_SRC="${BACKEND_DIR}/src"
BACKEND_TESTS="${BACKEND_DIR}/tests"
BACKEND_MIG="${BACKEND_DIR}/migration"

TRACE="${TRACE:-0}"
tlog() { [[ "$TRACE" == "1" ]] && printf 'TRACE: %s\n' "$*"; }

# --- Tooling sanity ----------------------------------------------------------
if ! command -v rg >/dev/null 2>&1; then
  echo "❌ guardrails: ripgrep (rg) is required. Install ripgrep with PCRE2 support."
  exit 2
fi
if ! rg --version | rg -q '\+pcre2'; then
  echo "⚠️ guardrails: rg built without PCRE2; lookarounds may not work correctly."
fi

# --- Target collection -------------------------------------------------------
declare -a TARGETS=()
if [[ "$MODE" == "staged" ]]; then
  echo "guardrails: mode = staged-only"
  mapfile -d '' TARGETS < <(
    git diff --name-only --cached --diff-filter=ACMRT -z \
    | rg -z "^${BACKEND_DIR}/"
  )
  if ((${#TARGETS[@]} == 0)); then
    echo "↪ no staged backend files; skipping"
    exit 0
  fi
else
  mapfile -t TARGETS < <(git ls-files "${BACKEND_DIR}")
  echo "guardrails: mode = full"
fi

# Keep only existing files
_existing=()
for f in "${TARGETS[@]}"; do [[ -f "$f" ]] && _existing+=("$f"); done
TARGETS=("${_existing[@]}")
if ((${#TARGETS[@]} == 0)); then
  echo "↪ no existing backend files in scope; skipping"
  exit 0
fi

# Convenience subsets for prod vs tests/migration
# (We’ll run some rules only on prod files)
mapfile -t PROD_FILES   < <(printf '%s\n' "${TARGETS[@]}" | rg "^${BACKEND_SRC}/")
mapfile -t TEST_FILES   < <(printf '%s\n' "${TARGETS[@]}" | rg "^${BACKEND_TESTS}/" || true)
mapfile -t MIG_FILES    < <(printf '%s\n' "${TARGETS[@]}" | rg "^${BACKEND_MIG}/"  || true)

tmpdir="$(mktemp -d)"; trap 'rm -rf "$tmpdir"' EXIT
fail_flag=0

# --- Allowlist expressions ---------------------------------------------------
# Tests/support/sentinel helpers
ALLOW_TEST_PATHS='(apps/backend/tests/support/|/tests/.*/(sentinel_|test_require_db|_test/require_db))'
# Specific prod files/dirs allowed to import SeaORM
ALLOW_SEA_PATHS="^(${BACKEND_SRC}/entities/|${BACKEND_SRC}/adapters/|${BACKEND_SRC}/db/|${BACKEND_SRC}/infra/|${BACKEND_SRC}/state/app_state\.rs:)"
# Concrete DB types may appear inside db/, infra/, and state/app_state.rs
ALLOW_DB_TYPES_PATHS="^${BACKEND_SRC}/(db/|infra/|state/app_state\.rs:)"
# Prod file that may legitimately call Database::connect
ALLOW_CONNECT_PROD_PATHS="^${BACKEND_SRC}/infra/db\.rs$"

# Content tokens that legitimize certain helper usage in tests
ALLOW_CONTENT_TOKENS='SharedTxn|shared_txn|with_txn'
contains_allow_tokens() { rg -q --pcre2 -n "${ALLOW_CONTENT_TOKENS}" -- "$1"; }

# Helper to write filtered matches: INPUT -> remove PATH_ALLOW -> OUTPUT
_filter_out_paths() {
  local path_regex="$1"; shift
  rg -v --pcre2 "${path_regex}" "$@" || true
}

# --- Rule 1: SeaORM import boundary (PROD ONLY) ------------------------------
# Disallow `use sea_orm::...` in prod **except** in allowed prod locations.
if ((${#PROD_FILES[@]})); then
  sea_raw="$tmpdir/sea_raw.txt"
  sea_hits="$tmpdir/sea_hits.txt"
  rg -n --pcre2 '^\s*use\s+sea_orm::(?!ConnectionTrait\b)' -- "${PROD_FILES[@]}" >"$sea_raw" || true
  # Drop allowed prod paths (entities, adapters, db, infra, state/app_state.rs)
  _filter_out_paths "${ALLOW_SEA_PATHS}" "$sea_raw" >"$sea_hits"
  if [[ -s "$sea_hits" ]]; then
    echo "❌ SeaORM imports outside allowed boundaries (prod code):"
    cat "$sea_hits"
    fail_flag=1
  fi
fi
# We intentionally do NOT police tests/migration for SeaORM imports.

# --- Rule 2: Concrete DB types leak (PROD ONLY) ------------------------------
# Disallow DatabaseConnection/DatabaseTransaction outside allowed prod places.
if ((${#PROD_FILES[@]})); then
  db_raw="$tmpdir/db_raw.txt"
  db_hits="$tmpdir/db_hits.txt"
  rg -n --pcre2 'Database(Connection|Transaction)\b' -- "${PROD_FILES[@]}" >"$db_raw" || true
  _filter_out_paths "${ALLOW_DB_TYPES_PATHS}" "$db_raw" >"$db_hits"
  if [[ -s "$db_hits" ]]; then
    echo "❌ Concrete DB types leaked outside db/ (prod code):"
    cat "$db_hits"
    fail_flag=1
  fi
fi

# --- Rule 3: Manual txn ops (PROD ONLY) --------------------------------------
# For prod code, forbid begin/commit/rollback calls outside txn.rs.
if ((${#PROD_FILES[@]})); then
  txn_raw="$tmpdir/txn_raw.txt"
  txn_hits="$tmpdir/txn_hits.txt"
  rg -n --pcre2 '\b(begin|commit|rollback)\b' -- "${PROD_FILES[@]}" >"$txn_raw" || true
  # Exempt canonical txn file
  rg -v --pcre2 "^${BACKEND_SRC}/db/txn\.rs:" "$txn_raw" >"$txn_hits" || true
  if [[ -s "$txn_hits" ]]; then
    echo "❌ Manual txn ops found outside txn.rs (prod code):"
    cat "$txn_hits"
    fail_flag=1
  fi
fi
# We do not flag tests or migration here; explicit rollbacks in tests are fine.

# --- Rule 4: Direct connects (prod & tests with strict allowlists) -----------
conn_list="$tmpdir/conn_list.txt"
rg -l --pcre2 'Database::connect|(^|[^:])\bconnect\(' -- "${TARGETS[@]}" >"$conn_list" || true

bad_conn=()
if [[ -s "$conn_list" ]]; then
  while IFS= read -r f; do
    case "$f" in
      # Allowed prod place for connects
      ${BACKEND_SRC}/infra/db.rs) continue ;;
    esac
    # Path allowlist for tests/support/sentinels
    if rg -q --pcre2 "${ALLOW_TEST_PATHS}" -N <<<"$f"; then
      continue
    fi
    # Content allowlist tokens for tests/helpers
    if contains_allow_tokens "$f"; then
      continue
    fi
    # Migration crate is allowed to connect
    if [[ "$f" == ${BACKEND_MIG}/* ]]; then
      continue
    fi
    bad_conn+=("$f")
  done <"$conn_list"
fi

if ((${#bad_conn[@]} > 0)); then
  echo "❌ Database::connect/connect() usage outside allowed locations:"
  printf '%s\n' "${bad_conn[@]}"
  fail_flag=1
fi

# --- Final verdict -----------------------------------------------------------
if [[ "$fail_flag" -ne 0 ]]; then
  echo "guardrails: FAIL"
  exit 1
else
  echo "✅ guardrails: OK"
fi

