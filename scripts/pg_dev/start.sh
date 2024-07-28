# shellcheck shell=bash

# Create database cluster
mkdir $PGDATA && initdb --auth-local=trust --auth-host=trust

# Start server
pg_ctl --log="$PGDATA/db.log" --options="-p $PGPORT -c unix_socket_directories='$PGDATA'" start || cat "$PGDATA/db.log"
echo "You can shut down the ${PGDATABASE} database with 'just pg_stop'"
