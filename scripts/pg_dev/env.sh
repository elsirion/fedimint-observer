# shellcheck shell=bash
set -o noclobber -o nounset

# shellcheck disable=SC2155
export PGDATA="$(pwd)/.pg_dev"
# shellcheck disable=SC2155
export PGUSER="$(whoami)"
export PGPORT="5432"
export PGHOST="$PGDATA"
export PGDATABASE="postgres"

# shellcheck disable=SC2139
alias pg="psql -p $PGPORT --dbname=${PGDATABASE} --host=$PGDATA"
