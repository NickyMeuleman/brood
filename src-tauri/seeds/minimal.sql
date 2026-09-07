-- Currencies
INSERT INTO
  currency (code)
VALUES
  ('EUR'),
  ('USD');

-- Baseline Tax Parameters (Current Year: 2026)
INSERT INTO
  cgt_parameters (
    tax_year,
    exemption_base_eur,
    exemption_cap_eur,
    carryforward_cap_eur
  )
VALUES
  (
    2026,
    '10000.00',
    '15000.00',
    '1000.00'
  );

INSERT INTO
  cgt_exemption_usage (
    tax_year,
    carryforward_in_eur,
    is_confirmed
  )
VALUES
  (2026, '0.00', 0);
