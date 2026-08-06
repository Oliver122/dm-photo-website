#!/bin/sh
set -e
# Persist data under the working directory (relative paths: data/app.db, data/ingest).
# Bind mounts / named volumes are often root-owned on first start.
mkdir -p data/ingest
chown -R app:app data
exec setpriv --reuid=10001 --regid=10001 --init-groups -- "$@"
