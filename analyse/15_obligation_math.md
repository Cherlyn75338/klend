### 15 — Obligation Math Deep Dive (`state/obligation.rs`)

Scope: User-level borrowing and collateral accounting, LTV computation, interest accrual per-borrow, and state transitions during repay/withdraw.

LTV and Value Fields
- Stored values (scaled Fractions):
  - `deposited_value_sf`, `borrowed_assets_market_value_sf` (unadjusted),
  - `borrow_factor_adjusted_debt_value_sf` (adjusted), `allowed_borrow_value_sf`, `unhealthy_borrow_value_sf`.
- Computations:
  - `loan_to_value() = adjusted_debt / deposited_value`.
  - `no_bf_loan_to_value() = unadjusted_borrowed / deposited_value`.
  - `unhealthy_loan_to_value() = unhealthy / deposited_value`.
- Invariant target: post non-liquidation, `ltv < unhealthy_ltv` (see 10_invariants.md).

Borrow/Repay Mutations
- `ObligationLiquidity::borrow(borrow_amount)` adds to `borrowed_amount_sf`.
- `ObligationLiquidity::repay(settle_amount)` subtracts from `borrowed_amount_sf`.
- `Obligation::repay(settle_amount, liquidity_index)` removes the line on full repay and clears tier, else delegates to line-level repay.

Accrue Interest (per-borrow line)
- `ObligationLiquidity::accrue_interest(new_cumulative_borrow_rate: BigFraction)`:
  - Reads former cumulative rate; compares with new (U256 compare).
  - If new < old → error NegativeInterestRate.
  - If new > old → scales `borrowed_amount_sf` by factor `new/old`, updates stored cumulative.
  - Monotonicity enforced at per-borrow granularity.

Withdrawals
- `Obligation::withdraw(withdraw_amount, collateral_index)` either fully removes collateral line (and tier) or subtracts amount with overflow-checked math.
- `max_withdraw_value(...)` determines max USD value withdrawable given either LTV or liquidation-threshold target, with edge-case: zero LTV config → allow full market value.

Remaining Borrow Value
- `remaining_borrow_value()` saturating difference `allowed - adjusted_debt` converted back to Fraction.

Safety Observations / Recommendations
- Accrual monotonicity: Stronger enforcement than reserve path; good guard against negative cumulative rates.
- Ensure that handlers call accrual consistently before using LTV checks (see 05_instruction_checklist.md), to avoid stale values granting excess capacity.
- Consider adding explicit checked saturation on `repay` when settle_amount slightly exceeds due to rounding; the code uses plain `-` but `min(settle, borrowed)` is applied earlier at reserve calculation, not here. Guard upstream is likely sufficient, but add property tests.

Links
- Combined with 14_reserve_math.md for reserve-level accrual and fee updates, and 16_lending_operations.md for price refresh that feeds market values.


