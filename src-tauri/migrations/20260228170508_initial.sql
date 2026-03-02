-- ============================================================
-- Database: SQLite (STRICT tables)
-- REQUIRED connection settings (enforced in sqlx, not here):
--   PRAGMA foreign_keys = ON;
--   In sqlx, this is enabled during connection
--   SqliteConnectOptions::new().foreign_keys(true)
--
-- TABLE ORDER:
-- Tables are ordered so that each REFERENCES clause points to a table
-- already defined above it. The one exception is cash_transaction, which
-- forward-references fx_conversion
-- ============================================================
-- CIRCULAR FK NOTE: cash_transaction ↔ fx_conversion
--   cash_transaction.fee_fx_conversion_id → fx_conversion.id
--   fx_conversion.debit/credit_cash_transaction_id → cash_transaction.id
--
--   Resolution:
--   - cash_transaction is created first. SQLite allows referencing
--     fx_conversion before it exists at DDL time; FKs are only
--     checked at DML time when foreign_keys = ON.
--   - fx_conversion FKs to cash_transaction are DEFERRABLE INITIALLY
--     DEFERRED so the constraint is checked at COMMIT, not INSERT.
--   - Insert order within a transaction must always be:
--       1. cash_transaction (debit FX leg)
--       2. cash_transaction (credit FX leg)
--       3. fx_conversion
--       4. cash_transaction (fee leg, if any) ← sets fee_fx_conversion_id
-- ============================================================
CREATE TABLE currency (
  code TEXT NOT NULL PRIMARY KEY
) STRICT;

CREATE TABLE fund_family (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE broker (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE cgt_parameters (
  tax_year INTEGER PRIMARY KEY,
  exemption_base_eur TEXT NOT NULL,
  exemption_cap_eur TEXT NOT NULL,
  carryforward_cap_eur TEXT NOT NULL
) STRICT;

CREATE TABLE instrument (
  id INTEGER PRIMARY KEY,
  isin TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  issuer TEXT,
  instrument_type TEXT NOT NULL,
  replication TEXT,
  fsma_registered INTEGER NOT NULL DEFAULT 0, -- boolean: 0/1
  fund_family_id INTEGER REFERENCES fund_family (id),
  accumulating INTEGER NOT NULL DEFAULT 1, -- boolean: 0/1
  domicile TEXT,
  subject_to_cgt INTEGER NOT NULL DEFAULT 1, -- boolean: 0/1
  --
  CHECK (
    instrument_type IN (
      'ETF',
      'FUND',
      'STOCK',
      'BOND',
      'OTHER'
    )
  ),
  CHECK (
    replication IS NULL
    OR replication IN (
      'PHYSICAL',
      'SYNTHETIC'
    )
  ),
  CHECK (
    fsma_registered IN (0, 1)
  ),
  CHECK (
    accumulating IN (0, 1)
  ),
  CHECK (
    subject_to_cgt IN (0, 1)
  )
) STRICT;

CREATE TABLE cash (
  id INTEGER PRIMARY KEY,
  broker_id INTEGER NOT NULL REFERENCES broker (id),
  currency_code TEXT NOT NULL REFERENCES currency (code),
  --
  UNIQUE (
    broker_id,
    currency_code
  )
) STRICT;

CREATE TABLE fx_rate (
  id INTEGER PRIMARY KEY,
  date TEXT NOT NULL,
  currency TEXT NOT NULL REFERENCES currency (code),
  rate_to_eur TEXT NOT NULL,
  source TEXT NOT NULL,
  --
  UNIQUE (
    date,
    currency,
    source
  ),
  CHECK (
    source IN (
      'ECB',
      'NBB',
      'BROKER',
      'MANUAL'
    )
  )
) STRICT;

CREATE TABLE cgt_exemption_usage (
  tax_year INTEGER PRIMARY KEY REFERENCES cgt_parameters (tax_year),
  carryforward_in_eur TEXT NOT NULL,
  used_eur TEXT,
  carryforward_out_eur TEXT,
  is_confirmed INTEGER NOT NULL DEFAULT 0, -- boolean: 0/1
  --
  CHECK (
    is_confirmed IN (0, 1)
  )
) STRICT;

CREATE TABLE listing (
  id INTEGER PRIMARY KEY,
  instrument_id INTEGER NOT NULL REFERENCES instrument (id),
  exchange_mic TEXT NOT NULL,
  ticker TEXT NOT NULL,
  currency_code TEXT NOT NULL REFERENCES currency (code),
  settlement_currency_code TEXT REFERENCES currency (code),
  delisted_at TEXT,
  --
  -- settlement_currency_code is only populated when it differs from currency_code.
  -- If they are the same, the column must be NULL.
  CHECK (
    settlement_currency_code IS NULL
    OR settlement_currency_code != currency_code
  )
) STRICT;

-- Only active (non-delisted) listings must be unique per (instrument, exchange, ticker).
-- A ticker can be reused on the same exchange after a delisting (e.g. after a SYMBOL_CHANGE).
-- An inline UNIQUE without WHERE would block that valid case.
CREATE UNIQUE INDEX idx_listing_active ON listing (
  instrument_id,
  exchange_mic,
  ticker
)
WHERE
  delisted_at IS NULL;

CREATE TABLE broker_order (
  id INTEGER PRIMARY KEY,
  listing_id INTEGER NOT NULL REFERENCES listing (id),
  broker_id INTEGER NOT NULL REFERENCES broker (id),
  order_type TEXT NOT NULL,
  side TEXT NOT NULL,
  requested_quantity TEXT NOT NULL,
  limit_price TEXT,
  placed_at TEXT NOT NULL,
  cancelled_at TEXT,
  broker_order_ref TEXT,
  --
  CHECK (
    order_type IN ('MARKET', 'LIMIT')
  ),
  CHECK (
    side IN ('BUY', 'SELL')
  ),
  CHECK (
    limit_price IS NULL
    OR order_type = 'LIMIT'
  ), -- MARKET orders cannot have a limit_price
  CHECK (
    order_type != 'LIMIT'
    OR limit_price IS NOT NULL
  ), -- LIMIT orders must have a limit_price
  CHECK (
    broker_order_ref IS NULL
    OR broker_order_ref != ''
  )
) STRICT;

-- Partial unique index: only enforce uniqueness when broker_order_ref is set
-- SQLite treats each NULL as distinct in unique constraints, so without WHERE
-- multiple manually-entered NULL rows would be blocked incorrectly
CREATE UNIQUE INDEX idx_broker_order_ref ON broker_order (
  broker_id,
  broker_order_ref
)
WHERE
  broker_order_ref IS NOT NULL;

CREATE TABLE dividend (
  id INTEGER PRIMARY KEY,
  instrument_id INTEGER NOT NULL REFERENCES instrument (id),
  broker_id INTEGER NOT NULL REFERENCES broker (id),
  ex_date TEXT NOT NULL,
  pay_date TEXT NOT NULL,
  quantity_held TEXT NOT NULL,
  gross_per_unit TEXT NOT NULL,
  currency_code TEXT NOT NULL REFERENCES currency (code),
  --
  -- A broker pays at most one dividend per instrument per ex_date
  UNIQUE (
    instrument_id,
    broker_id,
    ex_date
  )
) STRICT;

CREATE TABLE trade (
  id INTEGER PRIMARY KEY,
  broker_order_id INTEGER REFERENCES broker_order (id),
  listing_id INTEGER NOT NULL REFERENCES listing (id),
  broker_id INTEGER NOT NULL REFERENCES broker (id),
  side TEXT NOT NULL,
  quantity TEXT NOT NULL,
  price TEXT NOT NULL,
  executed_at TEXT NOT NULL,
  settlement_cash_id INTEGER REFERENCES cash (id),
  settlement_date TEXT,
  broker_trade_ref TEXT,
  --
  CHECK (
    side IN ('BUY', 'SELL')
  ),
  CHECK (
    broker_trade_ref IS NULL
    OR broker_trade_ref != ''
  )
) STRICT;

CREATE INDEX idx_trade_executed_at ON trade (executed_at);

CREATE INDEX idx_trade_listing_id ON trade (listing_id);

-- Partial unique index: only enforce uniqueness when broker_trade_ref is set
CREATE UNIQUE INDEX idx_trade_broker_ref ON trade (
  broker_id,
  broker_trade_ref
)
WHERE
  broker_trade_ref IS NOT NULL;

CREATE TABLE trade_fee (
  id INTEGER PRIMARY KEY,
  trade_id INTEGER NOT NULL REFERENCES trade (id),
  fee_type TEXT NOT NULL,
  amount TEXT NOT NULL,
  currency_code TEXT NOT NULL REFERENCES currency (code),
  notes TEXT,
  --
  CHECK (
    fee_type IN (
      'BROKER',
      'TOB',
      'FX',
      'OTHER'
    )
  ),
  CHECK (
    fee_type != 'OTHER'
    OR (
      notes IS NOT NULL
      AND notes != ''
    )
  )
) STRICT;

CREATE INDEX idx_trade_fee_trade ON trade_fee (trade_id, fee_type);

-- ============================================================
-- corporate_action references instrument twice:
-- instrument_id = the instrument this CA applies to
-- target_instrument_id = the receiving instrument (MERGER_INTO / SPIN_OFF only)
-- ============================================================
CREATE TABLE corporate_action (
  id INTEGER PRIMARY KEY,
  instrument_id INTEGER NOT NULL REFERENCES instrument (id),
  action_type TEXT NOT NULL,
  effective_date TEXT NOT NULL,
  ratio_from TEXT NOT NULL,
  ratio_to TEXT NOT NULL,
  cost_basis_factor TEXT,
  target_instrument_id INTEGER REFERENCES instrument (id),
  spin_off_ratio TEXT,
  notes TEXT,
  --
  CHECK (
    action_type IN (
      'SPLIT',
      'REVERSE_SPLIT',
      'MERGER_INTO',
      'SPIN_OFF',
      'SYMBOL_CHANGE'
    )
  ),
  -- SYMBOL_CHANGE: ratio must be 1:1, no lot transformation
  CHECK (
    action_type != 'SYMBOL_CHANGE'
    OR (
      ratio_from = '1'
      AND ratio_to = '1'
    )
  ),
  -- SPIN_OFF: ratio_from:ratio_to must be 1:1 (parent quantity unchanged)
  CHECK (
    action_type != 'SPIN_OFF'
    OR (
      ratio_from = '1'
      AND ratio_to = '1'
    )
  ),
  -- SPIN_OFF: spin_off_ratio is required
  CHECK (
    action_type != 'SPIN_OFF'
    OR spin_off_ratio IS NOT NULL
  ),
  -- cost_basis_factor: only for SPIN_OFF, required for SPIN_OFF
  CHECK (
    action_type = 'SPIN_OFF'
    OR cost_basis_factor IS NULL
  ),
  CHECK (
    action_type != 'SPIN_OFF'
    OR cost_basis_factor IS NOT NULL
  ),
  -- target_instrument_id: only for MERGER_INTO and SPIN_OFF, cannot reference itself
  CHECK (
    action_type IN (
      'MERGER_INTO',
      'SPIN_OFF'
    )
    OR target_instrument_id IS NULL
  ),
  CHECK (
    target_instrument_id IS NULL
    OR target_instrument_id != instrument_id
  )
) STRICT;

CREATE INDEX idx_corporate_action_instrument ON corporate_action (
  instrument_id,
  effective_date
);

-- ============================================================
-- lot references itself via parent_lot_id for CA audit trail only.
-- See DBML for full lineage semantics.
-- ============================================================
CREATE TABLE lot (
  id INTEGER PRIMARY KEY,
  broker_id_at_acquisition INTEGER NOT NULL REFERENCES broker (id),
  instrument_id INTEGER NOT NULL REFERENCES instrument (id),
  source_trade_id INTEGER REFERENCES trade (id),
  source_ca_id INTEGER REFERENCES corporate_action (id),
  parent_lot_id INTEGER REFERENCES lot (id),
  qty_at_acquisition TEXT NOT NULL,
  price_currency_code TEXT NOT NULL REFERENCES currency (code),
  price_per_unit TEXT NOT NULL,
  --
  -- Exactly one source must be set
  CHECK (
    (
      source_trade_id IS NULL
    ) != (
      source_ca_id IS NULL
    )
  )
) STRICT;

CREATE INDEX idx_lot_fifo ON lot (instrument_id, id);

-- ============================================================
-- fee_fx_conversion_id references fx_conversion which does not
-- exist yet. SQLite allows forward references at DDL time;
-- the FK is checked at DML time when foreign_keys = ON.
-- ============================================================
CREATE TABLE cash_transaction (
  id INTEGER PRIMARY KEY,
  cash_id INTEGER NOT NULL REFERENCES cash (id),
  direction TEXT NOT NULL,
  amount TEXT NOT NULL,
  transacted_at TEXT NOT NULL,
  category TEXT NOT NULL,
  trade_id INTEGER REFERENCES trade (id),
  dividend_id INTEGER REFERENCES dividend (id),
  source_ca_id INTEGER REFERENCES corporate_action (id),
  description TEXT,
  fee_fx_conversion_id INTEGER REFERENCES fx_conversion (id), -- forward ref; fx_conversion created next
  --
  CHECK (
    direction IN ('CREDIT', 'DEBIT')
  ),
  CHECK (
    category IN (
      'TRADE',
      'FEE',
      'TAX',
      'DIVIDEND',
      'TRANSFER',
      'FX',
      'OTHER'
    )
  ),
  -- Bi-conditional: trade_id set iff TRADE
  CHECK (
    (
      trade_id IS NOT NULL
    ) = (category = 'TRADE')
  ),
  -- Bi-conditional: dividend_id set iff DIVIDEND
  CHECK (
    (
      dividend_id IS NOT NULL
    ) = (
      category = 'DIVIDEND'
    )
  ),
  -- source_ca_id only allowed for OTHER (cash-in-lieu); OTHER without it is fine
  CHECK (
    source_ca_id IS NULL
    OR category = 'OTHER'
  ),
  -- fee_fx_conversion_id only allowed for FEE; FEE without it is fine
  CHECK (
    fee_fx_conversion_id IS NULL
    OR category = 'FEE'
  ),
  -- Mutual exclusion: at most one link column set at a time
  CHECK (
    (
      CASE
        WHEN trade_id IS NOT NULL THEN 1
        ELSE 0
      END + CASE
        WHEN dividend_id IS NOT NULL THEN 1
        ELSE 0
      END + CASE
        WHEN source_ca_id IS NOT NULL THEN 1
        ELSE 0
      END + CASE
        WHEN fee_fx_conversion_id IS NOT NULL THEN 1
        ELSE 0
      END
    ) <= 1
  )
) STRICT;

CREATE INDEX idx_cash_transaction_balance ON cash_transaction (
  cash_id,
  transacted_at
);

-- ============================================================
-- FKs to cash_transaction are DEFERRABLE INITIALLY DEFERRED.
-- See circular FK note at top of file.
-- ============================================================
CREATE TABLE fx_conversion (
  id INTEGER PRIMARY KEY,
  debit_cash_transaction_id INTEGER NOT NULL UNIQUE,
  credit_cash_transaction_id INTEGER NOT NULL UNIQUE,
  rate TEXT NOT NULL,
  executed_at TEXT NOT NULL,
  --
  -- Deferred FKs: checked at COMMIT, not at INSERT.
  -- Required because cash_transaction.fee_fx_conversion_id creates a cycle.
  -- See circular FK note at top of file.
  CONSTRAINT fk_fx_debit FOREIGN KEY (
    debit_cash_transaction_id
  ) REFERENCES cash_transaction (id) DEFERRABLE INITIALLY DEFERRED,
  CONSTRAINT fk_fx_credit FOREIGN KEY (
    credit_cash_transaction_id
  ) REFERENCES cash_transaction (id) DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE sell_allocation (
  id INTEGER PRIMARY KEY,
  sell_trade_id INTEGER NOT NULL REFERENCES trade (id),
  origin_lot_id INTEGER NOT NULL REFERENCES lot (id),
  quantity TEXT NOT NULL,
  --
  UNIQUE (
    sell_trade_id,
    origin_lot_id
  )
) STRICT;

CREATE INDEX idx_sell_allocation_lot ON sell_allocation (origin_lot_id);

CREATE TABLE lot_close (
  id INTEGER PRIMARY KEY,
  lot_id INTEGER NOT NULL UNIQUE REFERENCES lot (id),
  source_ca_id INTEGER NOT NULL REFERENCES corporate_action (id),
  closed_at TEXT NOT NULL,
  notes TEXT
) STRICT;

CREATE TABLE tax_snapshot_2025 (
  id INTEGER PRIMARY KEY,
  instrument_id INTEGER NOT NULL REFERENCES instrument (id),
  broker_id INTEGER NOT NULL REFERENCES broker (id),
  qty_at_snapshot TEXT NOT NULL,
  hist_cost_per_unit_eur TEXT NOT NULL,
  snap_price_per_unit_eur TEXT NOT NULL,
  --
  UNIQUE (
    instrument_id,
    broker_id
  )
) STRICT;

CREATE TABLE tax_sell_allocation (
  id INTEGER PRIMARY KEY,
  sell_allocation_id INTEGER NOT NULL UNIQUE REFERENCES sell_allocation (id),
  sale_price_eur TEXT NOT NULL,
  buy_price_eur TEXT NOT NULL,
  taxable_gain_eur TEXT NOT NULL,
  tax_year INTEGER NOT NULL REFERENCES cgt_parameters (tax_year),
  computed_at TEXT NOT NULL,
  sale_fx_rate_id INTEGER REFERENCES fx_rate (id),
  buy_fx_rate_id INTEGER REFERENCES fx_rate (id)
) STRICT;

CREATE TABLE dividend_fee (
  id INTEGER PRIMARY KEY,
  dividend_id INTEGER NOT NULL REFERENCES dividend (id),
  fee_type TEXT NOT NULL,
  amount TEXT NOT NULL,
  currency_code TEXT NOT NULL REFERENCES currency (code),
  notes TEXT,
  --
  CHECK (
    fee_type IN (
      'WITHHOLDING_TAX',
      'BROKER_FEE',
      'FX',
      'OTHER'
    )
  ),
  CHECK (
    fee_type != 'OTHER'
    OR (
      notes IS NOT NULL
      AND notes != ''
    )
  )
) STRICT;

CREATE TABLE lot_transfer (
  id INTEGER PRIMARY KEY,
  lot_id INTEGER NOT NULL REFERENCES lot (id),
  from_broker_id INTEGER NOT NULL REFERENCES broker (id),
  to_broker_id INTEGER NOT NULL REFERENCES broker (id),
  transferred_at TEXT NOT NULL,
  notes TEXT,
  --
  CHECK (
    from_broker_id != to_broker_id
  )
) STRICT;

CREATE INDEX idx_lot_transfer_lot ON lot_transfer (
  lot_id,
  transferred_at
);

CREATE TABLE price_history (
  id INTEGER PRIMARY KEY,
  listing_id INTEGER NOT NULL REFERENCES listing (id),
  date TEXT NOT NULL,
  close TEXT NOT NULL,
  source TEXT NOT NULL,
  --
  UNIQUE (
    listing_id,
    date,
    source
  ),
  CHECK (
    source IN (
      'EXCHANGE',
      'BROKER',
      'MANUAL'
    )
  )
) STRICT;
