#!/usr/bin/env bash

# Usage:
# $ ./scripts/migrate_sqlite_to_postgres.sh <sqlite_db_file>

set -euo pipefail

if [ -z "$1" ]; then
  echo "Must pass sqlite db file as arg"
  exit 1
fi

SQLITE_DB_FILE="$1"
DUMP_FILE=sqlite_db_dump.sql

pg="psql --host=$PGHOST -p $PGPORT --dbname=${PGDATABASE}"

if pg_isready > /dev/null 2>&1; then
  output=$($pg -c '\d' 2>&1)
  if [[ "$output" == *"Did not find any relations."* ]]; then
    echo "No relations found in postgres db, starting migration"
  else
    echo "Relations found in postgres db"
    echo "Migration must run using an empty postgres instance"
    exit 1
  fi
else
  echo "Must start an empty instance of postgres with just pg_start"
  exit 1
fi

echo "Dumping sqlite db file"
sqlite3 "$SQLITE_DB_FILE" .dump > "$DUMP_FILE"
echo "Finished sqlite dump"

echo "Parsing dump file, converting syntax to postgres"

sed -i \
  -e 's/PRAGMA foreign_keys=OFF;//g' \
  -e 's/BLOB/BYTEA/g' \
  -e "s/X'\([^']*\)'/'\\\x\1'/g" \
  -e 's/amount_msat INTEGER/amount_msat BIGINT/g' \
  -e 's/timestamp INTEGER/timestamp TIMESTAMP/g' \
  -e 's/FOREIGN KEY (federation_id, txid) REFERENCES transactions(federation_id, txid),/FOREIGN KEY (federation_id, txid) REFERENCES transactions(federation_id, txid)/g' \
  -e 's/FOREIGN KEY (federation_id, ln_contract_id) REFERENCES ln_contracts(federation_id, contract_id)/--/g' \
  -e "s/VALUES(\([0-9]*\),\s*\([0-9]*\))/VALUES(\1, to_timestamp(\2))/g" \
  "$DUMP_FILE"

echo "Finished parsing dump file"

echo "Importing parsed dump file into postgres"
echo "This may take several minutes..."
time $pg < "$DUMP_FILE" > /dev/null
echo "Finished importing dump file into postgres, cleaning up"
rm  "$DUMP_FILE"
