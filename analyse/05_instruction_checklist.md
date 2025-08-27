### 05 — Instruction-by-Instruction Checklist (Condensed, actionable)

This distills the plan’s checklist into concrete verification points using 01–04 context.

#### Borrow (`handler_borrow_obligation_liquidity.rs`)
- Require fresh `Reserve` and `Obligation`; confirm macro or explicit freshness checks present.
- Validate destination token account is not reserve vault; enforce Token-2022 extension validation.
- Use current price and updated interest before LTV checks; enforce per-reserve/group borrow limits.
- After token transfer, run post-transfer vault/accounting reconciliation.

#### Repay (`handler_repay_obligation_liquidity.rs`, `handler_repay_and_withdraw_redeem.rs`)
- Source must not be reserve vault; version/status checks.
- Partial repay must accrue interest correctly and adjust variable/fixed components.
- In combo flow, ensure LTV check runs after repay effects are applied before any withdrawal.

#### Deposit/Withdraw Liquidity and Collateral
- For deposits: disallow vaults as user sources; validate versions/status; compute mint/burn via correct exchange rate.
- Enforce deposit/withdraw caps and update interval counters atomically with transfer.

#### Liquidate and Redeem
- Validate eligibility with `check_liquidate_obligation`; enforce liquidation bonus within configured bounds.
- Ensure repay amount ≤ max allowed; verify accounting after redeem.

#### Flash Borrow/Repay
- Enforce non-zero fee path; charge fee precisely; require exact repayment, no dust tolerance.
- Consider same-slot interest accrual interactions; assert debt cannot be reduced via ordering.

#### Refresh Reserve/Obligation
- Accrue interest once per slot; apply price updates; enforce max age gating.
- Reset/clear reserved fields as expected; set conservative flags on stale/invalid price.

#### Admin/Config Updates
- Only admin or pending-admin-apply; validate `UpdateReserveConfigValue::Full` and granular item bounds.
- Disallow dangerous curves/fees; ensure status transitions prevent borrow on Obsolete/Hidden.

Cross-reference: see `01_architecture_map.md` for module locations and `03_high_risk_areas.md` for risk rationales.


