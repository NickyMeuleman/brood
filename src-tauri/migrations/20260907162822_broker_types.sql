ALTER TABLE broker
ADD COLUMN broker_type TEXT CHECK (
  broker_type IS NULL
  OR broker_type IN (
    'REBEL',
    'MEDIRECT',
    'BOLERO',
    'SAXO'
  )
);

INSERT INTO
  broker (name, broker_type)
VALUES
  ('Re=Bel', 'REBEL'),
  (
    'MeDirect',
    'MEDIRECT'
  ),
  ('Bolero', 'BOLERO'),
  ('Saxo', 'SAXO')
ON CONFLICT (name) DO UPDATE
SET
  broker_type = excluded.broker_type;
