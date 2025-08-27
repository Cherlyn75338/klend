### 11 — Files to Prioritize (with rationale)

Priority tiers derived from risk and centrality to flows (see 01 and 03).

Tier 0 (must-review first)
- `programs/klend/src/state/reserve.rs`: interest, exchange rates, caps, fees, limits.
- `programs/klend/src/state/obligation.rs`: user debt/collateral, LTV, interest accrual.
- `programs/klend/src/utils/fraction.rs`: fixed-point math primitives and conversions.
- `programs/klend/src/lending_market/lending_operations.rs`: refresh + price ingestion.

Tier 1 (validation core and price adapters)
- `programs/klend/src/lending_market/lending_checks.rs`: account/token validation, post-transfer.
- `programs/klend/src/utils/prices/*`: oracle adapters and checks.

Tier 2 (instruction surfaces)
- `programs/klend/src/handlers/*` with emphasis on borrow/repay/liquidate/withdraw/deposit/flash/refresh.

Tier 3 (supporting logic)
- `programs/klend/src/utils/borrow_rate_curve.rs`, `utils/seeds.rs`, token utils.

Review order will follow Tier 0 → Tier 1 → Tier 2 → Tier 3, integrating tests after each tier.


