#!/bin/sh
set -e
# Container boot init (REQ-010): normalize paths, create persist dirs, fix ownership,
# verify writability, then drop privileges to app (uid 10001).

DATA_DIR=/app/data
INGEST_DIR=/app/data/ingest

# Legacy Compose defaults (`sqlite://data/app.db`) are URI-parsed as host=data
# path=/app.db → SQLite tries to create /app.db as non-root → code 14.
case "${DATABASE_URL:-}" in
  ""|sqlite://data/*|sqlite:data/*|sqlite:///data/*)
    export DATABASE_URL="sqlite:/app/data/app.db"
    echo "entrypoint: DATABASE_URL → ${DATABASE_URL}"
    ;;
esac

case "${ANALOG_INGEST_DIR:-}" in
  ""|data/*|./data/*)
    export ANALOG_INGEST_DIR="${INGEST_DIR}"
    echo "entrypoint: ANALOG_INGEST_DIR → ${ANALOG_INGEST_DIR}"
    ;;
esac

mkdir -p "${DATA_DIR}" "${INGEST_DIR}"

if ! chown -R app:app "${DATA_DIR}"; then
  echo "entrypoint: warn: chown ${DATA_DIR} failed (continuing)" >&2
fi
chmod -R u+rwX "${DATA_DIR}" 2>/dev/null || true

# Prove the app user can create the DB + WAL files before exec.
if ! setpriv --reuid=10001 --regid=10001 --init-groups -- sh -c "
  touch '${DATA_DIR}/.write_test' && rm -f '${DATA_DIR}/.write_test'
"; then
  echo "entrypoint: error: ${DATA_DIR} is not writable by app (uid 10001)" >&2
  ls -la "${DATA_DIR}" >&2 || true
  ls -la /app >&2 || true
  id >&2 || true
  exit 1
fi

echo "entrypoint: data ok DATABASE_URL=${DATABASE_URL} ANALOG_INGEST_DIR=${ANALOG_INGEST_DIR}"
exec setpriv --reuid=10001 --regid=10001 --init-groups -- "$@"
