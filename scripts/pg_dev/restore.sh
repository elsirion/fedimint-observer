#!/usr/bin/env bash

set -euo pipefail

if [ -z "$1" ]; then
  echo "Must pass backup file as arg"
  exit 1
fi

BACKUP_FILE="$1"

pg="psql --host=$PGHOST -p $PGPORT --dbname=${PGDATABASE}"

if pg_isready > /dev/null 2>&1; then
  output=$($pg -c '\d' 2>&1)
  if [[ "$output" == *"Did not find any relations."* ]]; then
    echo "No relations found in postgres db, restoring"
  else
    echo "Relations found in postgres db"
    echo "Restore must run using an empty postgres instance"
    exit 1
  fi
else
  echo "Must start an empty instance of postgres with just pg_start"
  exit 1
fi

time $pg < "$BACKUP_FILE" > /dev/null
echo "Restore complete"
