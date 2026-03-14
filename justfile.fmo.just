pg_start:
  ./scripts/pg_dev/start.sh

pg_stop:
  ./scripts/pg_dev/stop.sh

pg_backup:
  ./scripts/pg_dev/backup.sh

pg_restore BACKUP_FILE:
  ./scripts/pg_dev/restore.sh {{BACKUP_FILE}}

serve_frontend:
  RUSTFLAGS=--cfg=web_sys_unstable_apis trunk serve fmo_frontend/index.html

# run the backend server
run *ARGS:
  cargo run -p fmo_server {{ARGS}}
