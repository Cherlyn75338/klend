### 16 — Lending Operations Deep Dive (`lending_market/lending_operations.rs`)

Scope: Refresh flows, price age handling, borrow/repay/deposit/withdraw orchestration, and invariant checks that tie state updates together.

Price Handling and Refresh
- `refresh_reserve(reserve, clock, price_opt, referral_fee_bps)`:
  - Always accrues interest first: `reserve.accrue_interest(clock.slot, referral_fee_bps)`.
  - If `price_opt` provided: sets `market_price_sf` and timestamp; stores `status` for last_update.
  - If no price and `!is_saved_price_age_valid(...)`: sets empty `PriceStatusFlags` (conservative).
  - Else: leaves price_status unchanged (None). Then `reserve.last_update.update_slot(slot, price_status)`.
- `is_saved_price_age_valid(reserve, now)` and `is_price_refresh_needed(reserve, market, now)` implement max-age and early-refresh thresholds (trigger percentage of max age).

Freshness Gating in Ops
- Most mutating ops require non-stale state with specific `PriceStatusFlags`:
  - Deposit reserve liquidity: requires `PriceStatusFlags::NONE` on reserve.
  - Borrow: requires `PriceStatusFlags::ALL_CHECKS` on borrow reserve and fully refreshed obligation.
  - Withdraw collateral: requires `NONE` if no borrows; else `ALL_CHECKS` for both reserve and obligation.
  - Repay: requires `NONE` for both reserve and obligation.
  - Redeem fees/collateral: requires `NONE` on reserve.
- This matches 02/05 guidance: sensitive price-dependent ops enforce freshness stringently.

Borrow Flow Highlights
- Checks min amount, stale reserve, market-level borrowing disabled, elevation group constraints, and per-reserve borrow limit.
- Computes remaining user capacity via `obligation.remaining_borrow_value()` and reserve headroom.
- Calls `reserve.calculate_borrow(...)` which computes fees and borrow amount (inclusive/exclusive path).
- Applies utilization limit gating after borrow to prevent exceeding configured utilization.
- Updates obligation: adds a borrow line with current cumulative rate; handles referrer fees into reserve available liquidity.
- Invariants: `post_borrow_obligation_invariants(...)` called.

Repay Flow
- Ensures freshness; accrues per-line interest using reserve’s cumulative rate.
- Uses `reserve.calculate_repay(...)` with ceil on `repay_amount` to avoid dust.
- Updates caps accounting and obligation/reserve states; runs `post_repay_obligation_invariants`.

Collateral Deposit/Withdraw
- Deposit path checks freshness and e-mode restrictions; adds or updates a collateral line; runs invariant checks.
- Withdraw path computes max withdraw via `Obligation::max_withdraw_value`, handling `u64::MAX` and partial-withdraw ratio math with `div_ceil` to avoid exceeding caps; enforces `WithdrawTooLarge` if exceeding.

Price to Value Conversions
- Market values are derived via helpers and by multiplying by `market_price` with mint factor, e.g., `liquidity_amount.mul(market_price).div(mint_factor)`.

Oracle and Staleness Risk Review
- If no fresh price is provided to `refresh_reserve` but the saved price is still within max age, `price_status` can remain unchanged (None) and the previously saved price persists. Sensitive ops later check `last_update.is_stale(slot, required_flags)`, ensuring correct gating.
- `UpdateTokenInfoPriceMaxAge` paths exist to change thresholds; misconfiguration can widen windows—covered by admin checks and tests.

Recommendations
- Confirm all handler entrypoints that rely on price call `refresh_*` or require `is_stale(..., ALL_CHECKS) == false` consistently; current checks appear in-place for borrow/withdraw.
- Consider logging when `price_status` becomes empty due to staleness to aid off-chain monitoring.

Links
- See 14_reserve_math.md and 15_obligation_math.md for math invariants; 17_lending_checks.md for token/accounting enforcement.


