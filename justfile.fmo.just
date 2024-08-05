pg_start:
  ./scripts/pg_dev/start.sh

pg_stop:
  ./scripts/pg_dev/stop.sh

serve_frontend:
  cd frontend && RUSTFLAGS=--cfg=web_sys_unstable_apis trunk serve
