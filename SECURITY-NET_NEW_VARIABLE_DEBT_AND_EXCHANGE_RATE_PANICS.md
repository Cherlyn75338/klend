### Security Analysis: net_new_variable_debt underflow and Exchange-Rate panics

Scope: `programs/klend/src/state/reserve.rs`, `programs/klend/src/lending_market/lending_operations.rs`, `programs/klend/src/utils/fraction.rs` and `programs/klend/src/lending_market/withdrawal_cap_operations.rs`.

This document confirms exploitability, details full impact, and presents realistic attack scenarios for two confirmed issues:

- net_new_variable_debt underflow during interest accrual and fee calculation
- panic/expect usage in `CollateralExchangeRate` conversions (deposit/redeem paths)

It also explains why other identified concerns are not exploitable for value theft given current guards, while still recommending hardening.

---

### 1) net_new_variable_debt_f underflow (per‑tx DoS)

Relevant code (interest accrual and fees):
```651:707:programs/klend/src/state/reserve.rs
fn compound_interest(
    &mut self,
    current_borrow_rate: Fraction,
    host_fixed_interest_rate: Fraction,
    slots_elapsed: u64,
    protocol_take_rate: Fraction,
    referral_rate: Fraction,
) -> LendingResult<()> {
    let previous_cumulative_borrow_rate = BigFraction::from(self.cumulative_borrow_rate_bsf);
    let previous_debt_f = Fraction::from_bits(self.borrowed_amount_sf);
    let acc_protocol_fees_f = Fraction::from_bits(self.accumulated_protocol_fees_sf);

    let compounded_interest_rate =
        approximate_compounded_interest(current_borrow_rate + host_fixed_interest_rate, slots_elapsed);
    let compounded_fixed_rate = approximate_compounded_interest(host_fixed_interest_rate, slots_elapsed);

    let new_cumulative_borrow_rate = previous_cumulative_borrow_rate * BigFraction::from(compounded_interest_rate);
    let new_debt_f = previous_debt_f * compounded_interest_rate;

    let fixed_host_fee = (previous_debt_f * compounded_fixed_rate) - previous_debt_f;
    let net_new_variable_debt_f = new_debt_f - previous_debt_f - fixed_host_fee;

    let variable_protocol_fee_f = net_new_variable_debt_f * protocol_take_rate;
    let absolute_referral_rate = protocol_take_rate * referral_rate;
    let max_referrers_fees_f = net_new_variable_debt_f * absolute_referral_rate;

    let new_acc_protocol_fees_f =
        acc_protocol_fees_f + fixed_host_fee + variable_protocol_fee_f - max_referrers_fees_f;

    self.cumulative_borrow_rate_bsf = new_cumulative_borrow_rate.into();
    self.pending_referrer_fees_sf += max_referrers_fees_f.to_bits();
    self.accumulated_protocol_fees_sf = new_acc_protocol_fees_f.to_bits();
    self.borrowed_amount_sf = new_debt_f.to_bits();
    self.absolute_referral_rate_sf = absolute_referral_rate.to_bits();

    Ok(())
}
```

Key facts:
- `Fraction` is an unsigned fixed type `U68F60` (`programs/klend/src/utils/fraction.rs`), so negative values are not representable.
- Workspace enforces overflow checks in release, and the `fixed` crate has debug assertions enabled:
```1:13:/workspace/Cargo.toml
[profile.release]
overflow-checks = true
[profile.release.package.fixed]
debug-assertions = true
overflow-checks = true
```

Impact mechanics:
- If `new_debt_f - previous_debt_f < fixed_host_fee`, then `new_debt_f - previous_debt_f - fixed_host_fee` underflows the unsigned fixed type, triggering a panic (transaction abort) due to overflow checks. There is no clamp/require around this subtraction.
- This occurs before any capping logic for referrer payouts in `lending_operations.rs`, so the entire accrual/refresh fails for the slot.

What this is NOT:
- It does NOT silently reduce protocol fees or create negative values on-chain; the program aborts before writing state.

Realistic trigger:
- Consider a reserve with small `previous_debt_f` and a nonzero `host_fixed_interest_rate_bps`. If `slots_elapsed` is large enough that the fixed component `(previous_debt_f * compounded_fixed_rate - previous_debt_f)` dominates the total interest delta `new_debt_f - previous_debt_f`, the computed `net_new_variable_debt_f` becomes negative. This can happen if host fixed rate is configured and compounding approximation creates a slight mismatch between total and fixed components over long durations. Any client calling `refresh_reserve` (or any op that calls it) in that state would consistently fail until another accrual occurs with different parameters (or code is fixed).

Attack scenario (DoS):
1) An attacker monitors a reserve where `host_fixed_interest_rate_bps > 0` and utilization/rates make the variable component small.
2) They avoid interacting to allow many slots to pass (accumulating large `slots_elapsed`).
3) They submit an instruction that calls `refresh_reserve` (`lending_operations::refresh_reserve`, which unconditionally calls `reserve.accrue_interest` first):
```42:76:programs/klend/src/lending_market/lending_operations.rs
reserve.accrue_interest(slot, referral_fee_bps)?;
... reserve.last_update.update_slot(slot, price_status);
```
4) Inside `accrue_interest`, `compound_interest` panics on underflow; the tx aborts. The reserve remains unrefreshed in that slot.
5) Repeating this at opportune times can cause repeated failures for operations that depend on a successful refresh in the slot (borrows/deposits that require non-stale reserves), degrading liveness.

Blast radius:
- Affects any instruction path that calls `refresh_reserve` (deposit, borrow, repay, redeem, config update, batch refresh handlers). Many handlers call `refresh_reserve` up front.
- No user funds can be stolen; however, markets become intermittently inoperable until parameters shift or the issue is patched.

Mitigations present:
- None around the subtraction; no clamp or assert.
- Validations constrain rates magnitudes, but do not prevent the underflow.

Remediation:
- Clamp/require non-negativity before fee math, e.g.:
  - `let net_new_variable_debt_f = (new_debt_f - previous_debt_f - fixed_host_fee).max(Fraction::ZERO);`
  - Or return a program error if negative instead of panicking.

---

### 2) CollateralExchangeRate panics via expect()/panic! (per‑tx DoS)

Panicking code paths:
```895:913:programs/klend/src/state/reserve.rs
pub fn collateral_to_liquidity_ceil(&self, collateral_amount: u64) -> u64 {
    let collateral_amount_u256 = U256::from(collateral_amount);
    let liquidity_sbf = BigFraction::from(self.liquidity).0;
    let collateral_supply_u256 = U256::from(self.collateral_supply);

    let liquidity_ceil_sbf = collateral_amount_u256
        .checked_mul(liquidity_sbf)
        .and_then(|res| res.checked_add(collateral_supply_u256 - U256::one()))
        .and_then(|res| res.checked_div(collateral_supply_u256))
        .expect("collateral_to_liquidity_ceil: liquidity_amount overflow on calculation");

    let liquidity_ceil_bf = BigFraction(liquidity_ceil_sbf);

    let liquidity_ceil_f = Fraction::try_from(liquidity_ceil_bf).expect(
        "collateral_to_liquidity_ceil: liquidity_amount overflow on fraction conversion",
    );

    liquidity_ceil_f.to_ceil()
}
```
```920:979:programs/klend/src/state/reserve.rs
fraction_collateral_to_liquidity(...).expect("... overflow")
fraction_liquidity_to_collateral(...).expect("... overflow")
fraction_liquidity_to_collateral_ceil(...).expect("... overflow")
liquidity_to_collateral_fraction(...).expect("... overflow")
liquidity_to_collateral(...).panic! in unwrap_or_else on failed try_to_floor
```

Callers:
- Deposit flow:
```133:161:programs/klend/src/lending_market/lending_operations.rs
let deposit_result = reserve.compute_depositable_amount_and_minted_collateral(liquidity_amount)?;
... reserve.deposit_liquidity(deposit_result.liquidity_amount, deposit_result.collateral_amount)?;
```
where:
```192:205:programs/klend/src/state/reserve.rs
pub fn compute_depositable_amount_and_minted_collateral(&self, liquidity_amount: u64) -> Result<...> {
    let collateral_amount = self.collateral_exchange_rate().liquidity_to_collateral(liquidity_amount);
    let liquidity_amount_to_deposit = self.collateral_exchange_rate().collateral_to_liquidity_ceil(collateral_amount);
    require_gte!(liquidity_amount, liquidity_amount_to_deposit, LendingError::MathOverflow);
    Ok(...)
}
```
- Redeem and other operations also reach these conversion paths.

Impact mechanics:
- Any overflow in the U256 arithmetic or downcast will trigger `.expect(...)` panics, aborting the transaction.
- Inputs come from user‑provided `liquidity_amount` (deposit), the reserve’s `available_amount` (redeem ceiling), or computed amounts; extreme magnitudes can trigger the panic.

Realistic trigger:
- An attacker (or even a benign user) attempts to deposit an extremely large `liquidity_amount` near `u64::MAX`. While the arithmetic uses U256 internally, the downcast to `Fraction` can fail if the scaled result does not fit into `U68F60`, causing `.expect(...)` to panic.
- Similarly, `liquidity_to_collateral` calls `.try_to_floor::<u64>()` and panics when the downcast fails. This can be hit when exchange rate is large and `liquidity_amount` is large.

Blast radius:
- Per‑tx DoS for deposit/redeem and routines using these conversions.
- No state corruption; the transaction aborts.

Mitigations present:
- Internal arithmetic uses checked U256 ops prior to the first `.expect`, but failure still panics.
- No graceful error propagation at call sites.

Remediation:
- Replace `.expect(...)`/`panic!` with proper error returns (`LendingError::IntegerOverflow`) and propagate as `Result`.

---

### Considerations judged not exploitable for value theft

1) Reserve cumulative rate monotonicity (no explicit assert)
- Inputs guarantee non‑negative compounding factor; Obligation path enforces monotone updates; Reserve uses multiplication by a factor ≥ 1 for non‑negative rates.
- Risk: logic regression could go unnoticed. Recommend parity assert.

2) approximate_compounded_interest overflow/accuracy
- Rates are bounded by config; base is small. Overflow triggers a panic due to release overflow checks, not silent wrap.
- Risk: per‑tx DoS at extreme `slots_elapsed` or misconfiguration. Optional guard on max slots.

3) absolute_referral_rate bounds
- Validated inputs ensure `absolute_referral_rate <= protocol_take_rate` and ≤ 1; payouts are further capped by `pending_referrer_fees_sf`.
- Recommend asserting the bound at write time for defense‑in‑depth.

---

### End‑to‑end exploit walkthroughs (illustrative)

1) DoS via negative net_new_variable_debt_f
- Preconditions: Reserve with `host_fixed_interest_rate_bps > 0`; low variable utilization/rate; significant `slots_elapsed` since last refresh.
- Steps:
  - Attacker invokes any instruction that calls `refresh_reserve` (deposit/borrow/repay/redeem/config update/refresh handlers). See:
    ```42:76:programs/klend/src/lending_market/lending_operations.rs```
  - In `reserve.accrue_interest`, the subtraction `new_debt_f - previous_debt_f - fixed_host_fee` underflows (unsigned `Fraction`), triggering a panic.
  - Tx aborts; reserve remains stale; subsequent operations in same slot fail.
- Repetition: Attacker repeats to degrade service until a different slot or conditions change.

2) DoS via exchange‑rate panics in deposit
- Preconditions: None special beyond large amounts.
- Steps:
  - Attacker calls deposit with very large `liquidity_amount`.
  - `compute_depositable_amount_and_minted_collateral` calls `liquidity_to_collateral(liquidity_amount)`, which can panic inside `.try_to_floor().unwrap_or_else(|| panic!(...))` on overflow.
  - Tx aborts cleanly.

---

### Recommended hardening changes

- In `compound_interest`:
  - Clamp: `net_new_variable_debt_f = max(net_new_variable_debt_f, Fraction::ZERO)` before fee computations; or return a program error if negative.
  - Assert: `new_cumulative_borrow_rate >= previous_cumulative_borrow_rate`.
  - Assert: `absolute_referral_rate <= protocol_take_rate` before writing.

- In `CollateralExchangeRate` and `utils/fraction.rs`:
  - Replace all `.expect(...)`/`panic!` with `Result<_, LendingError::IntegerOverflow>` and propagate errors.

- Optional:
  - Guard `slots_elapsed` to an upper bound per refresh to keep Taylor terms safe.

---

### Why these issues don't enable fund theft

- All critical arithmetic uses unsigned fixed types with release overflow checks enabled. When an invalid intermediate would occur (negative or out of range), the program aborts rather than producing a malformed state transition.
- Fee payouts are capped by accrued `pending_referrer_fees_sf`, and referral rates are bound by validated config.

The issues are liveness/DoS concerns and invariant clarity, not direct value extraction. However, production readiness requires eliminating all panics and adding explicit guards at critical boundaries.

