### 14 — Reserve Math Deep Dive (`state/reserve.rs`)

Scope: `Reserve`, `ReserveLiquidity`, `ReserveCollateral`, `CollateralExchangeRate`, fees, caps, interest compounding, and borrow/repay calculations.

Interest Accrual Path
- Entry: `Reserve::accrue_interest(current_slot, referral_fee_bps)` when `slots_elapsed > 0`.
- Computes:
  - `current_borrow_rate` from utilization via `borrow_rate_curve`.
  - `protocol_take_rate`, `referral_rate`, `host_fixed_interest_rate` as Fractions.
  - Calls `ReserveLiquidity::compound_interest` with rates and `slots_elapsed`.

ReserveLiquidity::compound_interest
- Inputs: `current_borrow_rate`, `host_fixed_interest_rate`, `slots_elapsed`, `protocol_take_rate`, `referral_rate`.
- Steps:
  - `previous_cumulative_borrow_rate` (BigFraction) and `previous_debt_f`.
  - `compounded_interest_rate = approximate_compounded_interest(current+host_fixed, slots)`.
  - `compounded_fixed_rate = approximate_compounded_interest(host_fixed, slots)`.
  - `new_cumulative_borrow_rate = prev_rate * compounded_interest_rate`.
  - `new_debt_f = previous_debt_f * compounded_interest_rate`.
  - Fees:
    - `fixed_host_fee = (previous_debt_f * compounded_fixed_rate) - previous_debt_f`.
    - `net_new_variable_debt_f = new_debt_f - previous_debt_f - fixed_host_fee`.
    - `variable_protocol_fee_f = net_new_variable_debt_f * protocol_take_rate`.
    - `absolute_referral_rate = protocol_take_rate * referral_rate`.
    - `max_referrers_fees_f = net_new_variable_debt_f * absolute_referral_rate`.
    - `new_acc_protocol_fees_f = acc_protocol_fees_f + fixed_host_fee + variable_protocol_fee_f - max_referrers_fees_f`.
  - Writes: cumulative rate, pending referrer fees, accumulated protocol fees, borrowed amount, and absolute referral rate.

Safety/Monotonicity Observations
- `approximate_compounded_interest` uses 3rd-order Taylor approximation beyond 4 slots; for small per-slot rates this is conservative and monotonic increasing for non-negative rates. For very large `elapsed_slots` or `rate`, need numeric bounds check (Phase 3).
- `ObligationLiquidity::accrue_interest` separately ensures new cumulative rate ≥ old, rejecting negative rates. Reserve path does not explicitly assert monotonicity of cumulative rate, but uses multiplicative update with positive compounding which should be ≥ previous.
- `net_new_variable_debt_f` can in theory go negative if rounding/approximation causes `new_debt_f - previous_debt_f < fixed_host_fee`. Needs bounds proof; add property test asserting non-negativity; clamp to zero if negative would avoid protocol fee underflow.

Borrow and Repay Calculations
- `calculate_borrow`:
  - For `u64::MAX`, computes max possible borrow subject to LTV/borrow-factor, remaining reserve borrow, and available liquidity; fees computed Inclusive; result floored for `borrow_amount` and adjusted.
  - For specific amount, computes Exclusive fees, adds to borrow amount, checks borrow-factor-adjusted value ≤ allowed; returns fees and receive amount.
- `calculate_repay`:
  - `settle_amount` is min(requested, borrowed), and `repay_amount = ceil(settle_amount)` to disallow dust under-repayment.

Collateral Exchange Rate
- `CollateralExchangeRate` provides both floor and ceil conversions.
- Ceil routines use U256 arithmetic and checked conversions; `fraction_*_ceil` adds `liquidity - DELTA` trick before division to bias upwards.

Caps and Limits
- `deposit_limit_crossed`, `borrow_limit_crossed` compare totals to config limits using Fraction arithmetic.
- Timestamp helpers set/clear crossed timestamps; interval authority enforcement lives in `withdrawal_cap_operations.rs` (to be reviewed later).

Potential Issues / Recommendations
- Add explicit non-negativity assertion for `net_new_variable_debt_f`; if negative, set to zero to avoid fees becoming negative.
- Consider capping `absolute_referral_rate <= protocol_take_rate` via checked math; currently it’s computed as a product and stored, with later logic assuming bounds.
- Ensure no panics: `collateral_to_liquidity_ceil` and other ceil paths use `expect()` on overflow. Prefer returning `LendingError::IntegerOverflow`.

Links
- See 13_fraction_math.md for arithmetic guarantees and rounding, and 15_obligation_math.md for per-obligation accrual enforcement.


