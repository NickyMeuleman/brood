-- ============================================================
-- Environment: development only. Never run in production.
-- ============================================================
-- SCENARIO: One broker, three ETFs, eight orders, nine trades,
-- one corporate action (IMIE 1:25 split Feb 2026). No sells.
--
-- Instruments:
--   SWRD  IE00BFY0GT14  State Street SPDR MSCI World UCITS ETF USD Unhedged
--                       Euronext Amsterdam (XAMS)
--   WEBN  IE0003XJA0J9  Amundi Prime All Country World UCITS ETF Acc
--                       Xetra (XETR)
--   IMIE  IE00B3YLTY66  SPDR MSCI All Country World Investable Market UCITS ETF Acc
--                       Euronext Paris (XPAR)
--
-- PARTIAL FILL:
--   Order 4: BUY 15 WEBN LIMIT @ 30.00
--     Fill 1 (trade 4):  10 shares @ 29.80  morning — carries broker fee
--     Fill 2 (trade 9):   5 shares @ 30.00  afternoon — TOB only, no broker fee
--   Order status is derived on the backend:
--     SUM(trade.quantity WHERE broker_order_id = 4) = 15 = requested_quantity → FILLED
--
-- For seeding purposes, manually insert ID fields
-- IDs are assigned by instrument group, not chronologically:
--   Orders/trades/lots 1-3   → SWRD
--   Orders/trades/lots 4-5   → WEBN (single-fill orders)
--   Orders/trades/lots 6-7   → IMIE (pre-split)
--   Lots 8-9                 → IMIE CA lots produced by the split (no corresponding trade)
--   Lot/trade 10             → IMIE post-split buy
--   Trade 9, Lot 11          → second fill of order 4 (the partially filled WEBN order).
--                              Trade 9 is inserted last in the trades block so both fills
--                              of order 4 sit adjacent in the SQL, at the cost of a
--                              non-sequential ID. Lot 11 follows from trade 9.
--
-- Timeline:
--   2023-04-15  Order 1 → Trade 1:  BUY 10 SWRD  @  80.00  → Lot 1
--   2024-03-10  Order 6 → Trade 6:  BUY  5 IMIE  @ 480.00  → Lot 6 (pre-split)
--   2024-07-15  Order 4 → Trade 4:  BUY 10 WEBN  @  29.80  → Lot 4 (fill 1 of 2)
--   2024-07-15  Order 4 → Trade 9:  BUY  5 WEBN  @  30.00  → Lot 11 (fill 2 of 2)
--   2024-08-20  Order 2 → Trade 2:  BUY 10 SWRD  @  95.00  → Lot 2
--   2025-05-20  Order 5 → Trade 5:  BUY 10 WEBN  @  35.00  → Lot 5
--   2025-06-20  Order 7 → Trade 7:  BUY  5 IMIE  @ 500.00  → Lot 7 (pre-split)
--   2025-09-10  Order 3 → Trade 3:  BUY  5 SWRD  @ 100.00  → Lot 3
--   --- 2025-12-31 snapshot ---
--   2026-02-15  SPLIT IMIE 1:25
--     Lot 6:   5 @ 480.00 → Lot 8: 125 @ 19.20  (basis: 2400 → 2400)
--     Lot 7:   5 @ 500.00 → Lot 9: 125 @ 20.00  (basis: 2500 → 2500)
--   2026-03-05  Order 8 → Trade 8:  BUY 10 IMIE  @  22.00  → Lot 10 (post-split)
--
-- Portfolio after seeding:
--   SWRD  25 shares  (lots 1-3)
--   WEBN  25 shares  (lots 4, 5, 11)
--   IMIE  260 shares (lots 8-9 from split + lot 10 from post-split buy)
--
-- Snapshot weighted averages (2025-12-31):
--   SWRD  hist = (10×80 + 10×95 + 5×100) / 25           = 2250/25  = 90.00
--   WEBN  hist = (10×29.80 + 5×30.00 + 10×35.00) / 25   =  798/25  = 31.92
--   IMIE  hist = (5×480 + 5×500) / 10  [pre-split units] = 4900/10  = 490.00
-- ============================================================
-- ============================================================
-- Reference data
-- ============================================================
INSERT INTO
  currency (code)
VALUES
  ('EUR');

INSERT INTO
  broker (id, name)
VALUES
  (1, 'Re=bel');

INSERT INTO
  cash (
    id,
    broker_id,
    currency_code
  )
VALUES
  (1, 1, 'EUR');

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

-- 2026 is the first year of meerwaardebelasting; no prior year to carry from.
INSERT INTO
  cgt_exemption_usage (
    tax_year,
    carryforward_in_eur,
    is_confirmed
  )
VALUES
  (2026, '0.00', 0);

-- ============================================================
-- Instruments
-- ============================================================
INSERT INTO
  instrument (
    id,
    isin,
    name,
    issuer,
    instrument_type,
    replication,
    fsma_registered,
    accumulating,
    domicile,
    subject_to_cgt
  )
VALUES
  (
    1,
    'IE00BFY0GT14',
    'State Street SPDR MSCI World UCITS ETF USD Unhedged',
    'SPDR',
    'ETF',
    'PHYSICAL',
    0,
    1,
    'IE',
    1
  ),
  (
    2,
    'IE0003XJA0J9',
    'Amundi Prime All Country World UCITS ETF Acc',
    'Amundi',
    'ETF',
    'PHYSICAL',
    0,
    1,
    'IE',
    1
  ),
  (
    3,
    'IE00B3YLTY66',
    'SPDR MSCI All Country World Investable Market UCITS ETF Acc',
    'SPDR',
    'ETF',
    'PHYSICAL',
    0,
    1,
    'IE',
    1
  );

-- ============================================================
-- Listings
-- ============================================================
INSERT INTO
  listing (
    id,
    instrument_id,
    exchange_mic,
    ticker,
    currency_code
  )
VALUES
  (
    1,
    1,
    'XAMS',
    'SWRD',
    'EUR'
  ), -- Euronext Amsterdam
  (
    2,
    2,
    'XETR',
    'WEBN',
    'EUR'
  ), -- Xetra
  (
    3,
    3,
    'XPAR',
    'IMIE',
    'EUR'
  );

-- Euronext Paris
-- ============================================================
-- Broker orders
-- ============================================================
INSERT INTO
  broker_order (
    id,
    listing_id,
    broker_id,
    order_type,
    side,
    requested_quantity,
    limit_price,
    placed_at
  )
VALUES
  -- SWRD orders (MARKET: limit_price is NULL)
  (
    1,
    1,
    1,
    'MARKET',
    'BUY',
    '10',
    NULL,
    '2023-04-15T09:30:00Z'
  ),
  (
    2,
    1,
    1,
    'MARKET',
    'BUY',
    '10',
    NULL,
    '2024-08-20T10:12:00Z'
  ),
  (
    3,
    1,
    1,
    'MARKET',
    'BUY',
    '5',
    NULL,
    '2025-09-10T11:03:00Z'
  ),
  -- WEBN order: LIMIT @ 30.00, fills in two parts
  (
    4,
    2,
    1,
    'LIMIT',
    'BUY',
    '15',
    '30.00',
    '2024-07-15T09:00:00Z'
  ),
  (
    5,
    2,
    1,
    'MARKET',
    'BUY',
    '10',
    NULL,
    '2025-05-20T14:28:00Z'
  ),
  -- IMIE orders (MARKET)
  (
    6,
    3,
    1,
    'MARKET',
    'BUY',
    '5',
    NULL,
    '2024-03-10T09:58:00Z'
  ),
  (
    7,
    3,
    1,
    'MARKET',
    'BUY',
    '5',
    NULL,
    '2025-06-20T11:18:00Z'
  ),
  (
    8,
    3,
    1,
    'MARKET',
    'BUY',
    '10',
    NULL,
    '2026-03-05T09:53:00Z'
  );

-- ============================================================
-- Trades  (all reference their originating broker_order)
-- ============================================================
INSERT INTO
  trade (
    id,
    broker_order_id,
    listing_id,
    broker_id,
    side,
    quantity,
    price,
    executed_at,
    settlement_cash_id,
    settlement_date
  )
VALUES
  -- SWRD
  (
    1,
    1,
    1,
    1,
    'BUY',
    '10',
    '80.00',
    '2023-04-15T09:32:00Z',
    1,
    '2023-04-17'
  ),
  (
    2,
    2,
    1,
    1,
    'BUY',
    '10',
    '95.00',
    '2024-08-20T10:14:00Z',
    1,
    '2024-08-22'
  ),
  (
    3,
    3,
    1,
    1,
    'BUY',
    '5',
    '100.00',
    '2025-09-10T11:05:00Z',
    1,
    '2025-09-12'
  ),
  -- WEBN fill 1 of 2: 10 shares, morning, carries broker fee
  (
    4,
    4,
    2,
    1,
    'BUY',
    '10',
    '29.80',
    '2024-07-15T09:45:00Z',
    1,
    '2024-07-17'
  ),
  -- WEBN single-fill order
  (
    5,
    5,
    2,
    1,
    'BUY',
    '10',
    '35.00',
    '2025-05-20T14:30:00Z',
    1,
    '2025-05-22'
  ),
  -- IMIE pre-split
  (
    6,
    6,
    3,
    1,
    'BUY',
    '5',
    '480.00',
    '2024-03-10T10:00:00Z',
    1,
    '2024-03-12'
  ),
  (
    7,
    7,
    3,
    1,
    'BUY',
    '5',
    '500.00',
    '2025-06-20T11:20:00Z',
    1,
    '2025-06-22'
  ),
  -- IMIE post-split
  (
    8,
    8,
    3,
    1,
    'BUY',
    '10',
    '22.00',
    '2026-03-05T09:55:00Z',
    1,
    '2026-03-07'
  ),
  -- WEBN fill 2 of 2: 5 shares, afternoon, hits limit price, no broker fee
  (
    9,
    4,
    2,
    1,
    'BUY',
    '5',
    '30.00',
    '2024-07-15T14:22:00Z',
    1,
    '2024-07-17'
  );

-- ============================================================
-- Trade fees
-- TOB: 0.35% on notional per fill
-- Broker fee: €1.00 flat, on first fill only for partial fills
-- ============================================================
INSERT INTO
  trade_fee (
    id,
    trade_id,
    fee_type,
    amount,
    currency_code
  )
VALUES
  -- Trade 1: 10 × 80.00  × 0.0035 = 2.80
  (
    1,
    1,
    'TOB',
    '2.80',
    'EUR'
  ),
  (
    2,
    1,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 2: 10 × 95.00  × 0.0035 = 3.33
  (
    3,
    2,
    'TOB',
    '3.33',
    'EUR'
  ),
  (
    4,
    2,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 3:  5 × 100.00 × 0.0035 = 1.75
  (
    5,
    3,
    'TOB',
    '1.75',
    'EUR'
  ),
  (
    6,
    3,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 4: WEBN fill 1 — 10 × 29.80 × 0.0035 = 1.04, carries broker fee
  (
    7,
    4,
    'TOB',
    '1.04',
    'EUR'
  ),
  (
    8,
    4,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 5: 10 × 35.00  × 0.0035 = 1.23
  (
    9,
    5,
    'TOB',
    '1.23',
    'EUR'
  ),
  (
    10,
    5,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 6:  5 × 480.00 × 0.0035 = 8.40
  (
    11,
    6,
    'TOB',
    '8.40',
    'EUR'
  ),
  (
    12,
    6,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 7:  5 × 500.00 × 0.0035 = 8.75
  (
    13,
    7,
    'TOB',
    '8.75',
    'EUR'
  ),
  (
    14,
    7,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 8: 10 × 22.00  × 0.0035 = 0.77
  (
    15,
    8,
    'TOB',
    '0.77',
    'EUR'
  ),
  (
    16,
    8,
    'BROKER',
    '1.00',
    'EUR'
  ),
  -- Trade 9: WEBN fill 2 — 5 × 30.00 × 0.0035 = 0.53, no broker fee
  (
    17,
    9,
    'TOB',
    '0.53',
    'EUR'
  );

-- ============================================================
-- Lots  (one per trade; lots 6+7 will be closed by the CA)
-- ============================================================
INSERT INTO
  lot (
    id,
    broker_id_at_acquisition,
    instrument_id,
    listing_id,
    source_trade_id,
    qty_at_acquisition,
    price_currency_code,
    price_per_unit
  )
VALUES
  -- SWRD
  (
    1,
    1,
    1,
    1,
    1,
    '10',
    'EUR',
    '80.00'
  ),
  (
    2,
    1,
    1,
    1,
    2,
    '10',
    'EUR',
    '95.00'
  ),
  (
    3,
    1,
    1,
    1,
    3,
    '5',
    'EUR',
    '100.00'
  ),
  -- WEBN: two lots from the partially filled order, one from single fill
  (
    4,
    1,
    2,
    2,
    4,
    '10',
    'EUR',
    '29.80'
  ), -- fill 1
  (
    5,
    1,
    2,
    2,
    5,
    '10',
    'EUR',
    '35.00'
  ),
  (
    11,
    1,
    2,
    2,
    9,
    '5',
    'EUR',
    '30.00'
  ), -- fill 2
  -- IMIE pre-split (closed by corporate action below)
  (
    6,
    1,
    3,
    3,
    6,
    '5',
    'EUR',
    '480.00'
  ),
  (
    7,
    1,
    3,
    3,
    7,
    '5',
    'EUR',
    '500.00'
  ),
  -- IMIE post-split trade-originated lot (post-2026)
  (
    10,
    1,
    3,
    3,
    8,
    '10',
    'EUR',
    '22.00'
  );

-- ============================================================
-- IMIE 1:25 split  (effective 2026-02-15)
-- ratio_from=1, ratio_to=25
-- ============================================================
INSERT INTO
  corporate_action (
    id,
    instrument_id,
    action_type,
    effective_date,
    ratio_from,
    ratio_to
  )
VALUES
  (
    1,
    3,
    'SPLIT',
    '2026-02-15',
    '1',
    '25'
  );

-- Retire the two pre-split lots
INSERT INTO
  lot_close (
    id,
    lot_id,
    source_ca_id,
    closed_at
  )
VALUES
  (
    1,
    6,
    1,
    '2026-02-15T00:00:00Z'
  ),
  (
    2,
    7,
    1,
    '2026-02-15T00:00:00Z'
  );

-- CA-originated lots with post-split quantities
-- qty   = old_qty × (ratio_to / ratio_from) = old_qty × 25
-- price = old_price × (ratio_from / ratio_to) = old_price / 25
-- Cost basis preserved: 125×19.20=2400=5×480 ✓  125×20.00=2500=5×500 ✓
INSERT INTO
  lot (
    id,
    broker_id_at_acquisition,
    instrument_id,
    listing_id,
    source_ca_id,
    parent_lot_id,
    qty_at_acquisition,
    price_currency_code,
    price_per_unit
  )
VALUES
  (
    8,
    1,
    3,
    3,
    1,
    6,
    '125',
    'EUR',
    '19.20'
  ), -- from lot 6
  (
    9,
    1,
    3,
    3,
    1,
    7,
    '125',
    'EUR',
    '20.00'
  );

-- from lot 7
-- ============================================================
-- Cash transactions  (settlement debits; all buys)
-- amount = quantity × price (gross; fees tracked in trade_fee)
-- ============================================================
INSERT INTO
  cash_transaction (
    id,
    cash_id,
    direction,
    amount,
    transacted_at,
    category,
    trade_id
  )
VALUES
  (
    1,
    1,
    'DEBIT',
    '800.00',
    '2023-04-17T00:00:00Z',
    'TRADE',
    1
  ),
  (
    2,
    1,
    'DEBIT',
    '950.00',
    '2024-08-22T00:00:00Z',
    'TRADE',
    2
  ),
  (
    3,
    1,
    'DEBIT',
    '500.00',
    '2025-09-12T00:00:00Z',
    'TRADE',
    3
  ),
  (
    4,
    1,
    'DEBIT',
    '298.00',
    '2024-07-17T00:00:00Z',
    'TRADE',
    4
  ), -- 10 × 29.80
  (
    5,
    1,
    'DEBIT',
    '350.00',
    '2025-05-22T00:00:00Z',
    'TRADE',
    5
  ),
  (
    6,
    1,
    'DEBIT',
    '2400.00',
    '2024-03-12T00:00:00Z',
    'TRADE',
    6
  ),
  (
    7,
    1,
    'DEBIT',
    '2500.00',
    '2025-06-22T00:00:00Z',
    'TRADE',
    7
  ),
  (
    8,
    1,
    'DEBIT',
    '220.00',
    '2026-03-07T00:00:00Z',
    'TRADE',
    8
  ),
  (
    9,
    1,
    'DEBIT',
    '150.00',
    '2024-07-17T00:00:00Z',
    'TRADE',
    9
  );

-- 5 × 30.00
-- ============================================================
-- Tax snapshot 2025  (as if received from Re=bel statement)
-- IMIE quantities are in pre-split (2025-era) units.
-- ============================================================
INSERT INTO
  tax_snapshot_2025 (
    id,
    instrument_id,
    broker_id,
    qty_at_snapshot,
    hist_cost_per_unit_eur,
    snap_price_per_unit_eur
  )
VALUES
  -- SWRD: hist = (10×80 + 10×95 + 5×100)/25 = 90.00
  (
    1,
    1,
    1,
    '25',
    '90.00',
    '105.00'
  ),
  -- WEBN: hist = (10×29.80 + 5×30.00 + 10×35.00)/25 = 31.92
  (
    2,
    2,
    1,
    '25',
    '31.92',
    '38.00'
  ),
  -- IMIE: hist = (5×480 + 5×500)/10 = 490.00  [pre-split units]
  (
    3,
    3,
    1,
    '10',
    '490.00',
    '520.00'
  );

-- ============================================================
-- Price history
-- 2025-12-31: must match snap_price_per_unit_eur above
-- 2026-05-01: recent prices for holdings view
--             IMIE in post-split units (~€20 range)
-- ============================================================
INSERT INTO
  price_history (
    id,
    listing_id,
    date,
    close,
    source
  )
VALUES
  (
    1,
    1,
    '2025-12-31',
    '105.00',
    'EXCHANGE'
  ), -- SWRD year-end
  (
    2,
    2,
    '2025-12-31',
    '38.00',
    'EXCHANGE'
  ), -- WEBN year-end
  (
    3,
    3,
    '2025-12-31',
    '520.00',
    'EXCHANGE'
  ), -- IMIE year-end (pre-split price)
  (
    4,
    1,
    '2026-05-01',
    '112.00',
    'EXCHANGE'
  ), -- SWRD recent
  (
    5,
    2,
    '2026-05-01',
    '41.00',
    'EXCHANGE'
  ), -- WEBN recent
  (
    6,
    3,
    '2026-05-01',
    '21.50',
    'EXCHANGE'
  );

-- IMIE recent (post-split units)
