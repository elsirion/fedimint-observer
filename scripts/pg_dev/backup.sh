#!/usr/bin/env bash

set -euo pipefail

mkdir -p db_backups
timestamp=$(date +'%Y-%m-%d_%H%M')
pg_dump --host="$PGHOST" -p "$PGPORT" --dbname="${PGDATABASE}" > "db_backups/${timestamp}_backup.sql"
