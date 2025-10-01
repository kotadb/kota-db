#!/usr/bin/env bash

set -euo pipefail

ENV_FILE="${1:-.env}"

if [[ ! -f "${ENV_FILE}" ]]; then
    echo "Environment file '${ENV_FILE}' not found." >&2
    exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
    echo "psql is required but was not found in PATH." >&2
    exit 1
fi

set -o allexport
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +o allexport

if [[ -z "${DATABASE_URL:-}" ]]; then
    echo "DATABASE_URL is not defined in '${ENV_FILE}'." >&2
    exit 1
fi

echo "Testing connection for DATABASE_URL from '${ENV_FILE}'"

export PGCONNECT_TIMEOUT="${PGCONNECT_TIMEOUT:-15}"

if psql --set ON_ERROR_STOP=1 "${DATABASE_URL}" -c 'SELECT 1;' >/dev/null; then
    echo "✅ Connection succeeded."
else
    echo "❌ Connection failed." >&2
    exit 1
fi
