### 03 — High-Risk Areas and Hypotheses

This expands Section 5 of the plan with concrete checks, exploit sketches, and evidence to gather during code review and testing.

#### 1) Interest and Fee Math
- Files: `state/reserve.rs::compound_interest`, `state/obligation.rs::accrue_interest`, `utils/fraction.rs`, `utils/borrow_rate_curve.rs`.
- Hypotheses to test:
  - Cumulative rates are non-decreasing even across slot boundaries and refresh interleavings.
  - No negative deltas in variable/fixed debt; fees are non-negative; referral fee bounded by protocol take rate.
  - Rounding cannot be exploited by repeated small operations (no dust farming).
- Actions:
  - Enumerate all cross-type arithmetic (U128/U256/BigFraction) and document rounding directions.
  - Property tests varying utilization, slots, and curve segments; assert monotonicity and non-negativity.

#### 2) Price Oracles and Staleness
- Files: `lending_market/lending_operations.rs::refresh_reserve`, `utils/prices/*`.
- Risks: Using saved price when age window still passes but is too permissive; missing conservative flags.
- Actions:
  - Map every handler path that reads price-dependent fields; confirm freshness gating or conservative block on missing prices.
  - Tests: simulate near-threshold staleness and ensure borrow/withdraw fail or are conservatively limited.

#### 3) LTV / Liquidation / E-Mode / Borrow Factors
- Files: `state/obligation.rs`, `state/liquidation_operations.rs`, `state/reserve.rs`, `state/lending_market.rs`.
- Risks: Borrow-factor adjusted values vs unhealthy thresholds drift; e-mode mixing bypasses limits; liquidation bonus bounds inconsistent.
- Actions: Cross-compare LTV computations pre/post repay/withdraw; assert `ltv <= unhealthy_ltv` for all non-liquidation flows.

#### 4) Collateral and Exchange Rates
- File: `state/reserve.rs`.
- Risks: Decimals extremes, exchange-rate rounding drift across mint/burn; stale exchange rate windows exploited.
- Actions: Range tests for assets with 0–12+ decimals; bound rounding error ≤ 1 wei per tx.

#### 5) Withdrawal/Borrow Caps and Limits
- Files: `state/reserve.rs`, `lending_market/withdrawal_cap_operations.rs`.
- Risks: Interval reset off-by-one; signed arithmetic underflow/overflow; bypass via interleaved combined instructions.
- Actions: Property tests around boundary timestamps and interleaved ops; assert never exceed per-interval caps.

#### 6) Flash Loans
- Files: `handlers/handler_flash_*`, `lending_checks.rs`.
- Risks: Zero-fee configuration path; rounding-based under-repayment; same-slot interest accrual edge cases.
- Actions: Tests that enforce minimum fee > 0 and exact repay with tolerance 0.

#### 7) Token Accounting vs Vault Balances
- File: `lending_checks.rs::post_transfer_vault_balance_liquidity_reserve_checks`.
- Risks: Missing invocation in some handlers; Token-2022 extensions that change transfer semantics.
- Actions: Grep for calls across all handlers; add tests with malicious extensions to ensure rejection.

#### 8) Admin and Config
- Files: `state/global_config.rs`, `handlers/handler_update_*`, `lending_market/config_items.rs`.
- Risks: Pending admin misuse; unsafe partial updates; fee collector redirection.
- Actions: Unit tests for each admin path and value range validations (borrow curve monotonicity, fee bounds).

#### 9) PDA, Seeds, and Account Validation
- Files: `utils/seeds.rs`, `lending_checks.rs`, handlers.
- Risks: Incorrect seeds, missing signer checks, remaining-accounts spoofing.
- Actions: Recompute PDAs in tests; negative tests for wrong bump/seed; enforce owner checks.

#### 10) Farms, Rewards, Socialize Loss
- Files: farms refresh handlers, `handler_socialize_loss.rs`.
- Risks: Adjustments altering debt/collateral not gated by invariants; arbitrary loss shifting.
- Actions: Validate invariant preservation after socialize-loss; cap adjustments bounded by config.

This list will drive targeted reads and tests in the next sections.


