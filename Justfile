default: dev

dev:
  pnpm tauri dev

build:
  # https://github.com/tauri-apps/tauri/issues/13113#issuecomment-3162433538
  NO_STRIP=true pnpm tauri build

# Add a SQLx migration file (timestamp_name.sql)
migrate-add name:
  cd src-tauri && sqlx migrate add {{name}}

# Apply SQLX migrations
migrate:
  cd src-tauri && sqlx migrate run

# Generate offline data for SQLx (.sqlx/)
prepare:
  cd src-tauri && cargo sqlx prepare -- --all-targets --all-features

db-reset:
  cd src-tauri && sqlx database drop
  cd src-tauri && sqlx database create
  just migrate
  just prepare
