#!/bin/sh
set -e
# Named volumes are root-owned on first mount; the app runs as uid 10001.
mkdir -p /data/ingest
chown -R app:app /data
exec setpriv --reuid=10001 --regid=10001 --init-groups -- "$@"
