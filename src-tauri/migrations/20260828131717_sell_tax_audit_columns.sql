ALTER TABLE tax_sell_allocation ADD COLUMN fotomoment_cost_eur TEXT;
ALTER TABLE tax_sell_allocation ADD COLUMN historical_cost_eur TEXT;
ALTER TABLE tax_sell_allocation ADD COLUMN pre2026_cost_basis_method TEXT
  CHECK (
    pre2026_cost_basis_method IS NULL
    OR pre2026_cost_basis_method IN ('FOTOMOMENT', 'HISTORICAL_ELECTED', 'FLOORED_AT_ZERO')
  );
ALTER TABLE tax_sell_allocation ADD COLUMN tax_snapshot_2025_id INTEGER
  REFERENCES tax_snapshot_2025 (id);
