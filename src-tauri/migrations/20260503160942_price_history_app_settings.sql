ALTER TABLE price_history ADD COLUMN open TEXT;
ALTER TABLE price_history ADD COLUMN high TEXT;
ALTER TABLE price_history ADD COLUMN low TEXT;
ALTER TABLE price_history ADD COLUMN volume INTEGER;

-- Generic key-value store for app-level settings.
-- example usages: last_sync_attempted_at, user preferences
CREATE TABLE app_settings (
  key   TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;
