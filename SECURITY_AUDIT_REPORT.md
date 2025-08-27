# Klend Security Audit Report

## Executive Summary

This comprehensive security audit of the Klend lending protocol has identified several critical and high-severity vulnerabilities that could lead to fund theft, insolvency, broken invariants, and systemic loss. The vulnerabilities span across mathematical operations, price oracle handling, liquidation logic, and administrative functions.

## Critical Vulnerabilities

### 1. **[CRITICAL] Integer Overflow in Interest Rate Calculation**

**Severity:** Critical  
**Affected Files:** `state/reserve.rs:1466-1467`  
**Impact:** Potential for interest rate manipulation leading to incorrect debt calculations

**Description:**
The `approximate_compounded_interest` function uses `wrapping_sub` operations which can cause integer underflow:

```rust
let exp_minus_one = exp.wrapping_sub(1);
let exp_minus_two = exp.wrapping_sub(2);
```

When `elapsed_slots` is 0 or 1, these operations will underflow, potentially causing incorrect interest calculations that could be exploited to reduce debt or increase collateral value incorrectly.

**Exploit Scenario:**
1. Attacker times operations when `elapsed_slots` would be 0 or 1
2. The underflow causes incorrect interest rate calculation
3. Debt is calculated incorrectly, allowing under-repayment

**Recommended Fix:**
Replace `wrapping_sub` with `saturating_sub` or add explicit checks:
```rust
let exp_minus_one = exp.saturating_sub(1);
let exp_minus_two = exp.saturating_sub(2);
```

### 2. **[CRITICAL] Price Staleness Window Exploitation**

**Severity:** Critical  
**Affected Files:** `lending_market/lending_operations.rs:83-89`, `state/last_update.rs`  
**Impact:** Ability to borrow/liquidate using stale prices

**Description:**
The price staleness check has a vulnerability window where operations can proceed with outdated prices:

```rust
pub fn is_saved_price_age_valid(reserve: &Reserve, current_ts: clock::UnixTimestamp) -> bool {
    current_ts.saturating_sub(price_last_updated_ts) < price_max_age
}
```

The function returns `true` when price age equals `price_max_age - 1`, allowing operations with nearly stale prices. Combined with the refresh trigger logic, there's a window where prices are "valid" but not fresh.

**Exploit Scenario:**
1. Monitor price feeds for significant movements
2. When favorable price exists, wait until just before staleness threshold
3. Execute large borrow/liquidation using the stale but "valid" price
4. Price updates after transaction, creating bad debt

**Recommended Fix:**
- Implement stricter freshness requirements for high-value operations
- Add price confidence checks beyond just age
- Require refresh for operations above certain thresholds

### 3. **[HIGH] Flash Loan Fee Bypass via Precision Loss**

**Severity:** High  
**Affected Files:** `handlers/handler_flash_repay_reserve_liquidity.rs`, `lending_market/lending_operations.rs:1673-1715`  
**Impact:** Potential to execute flash loans with reduced or zero fees

**Description:**
The flash loan fee calculation doesn't properly handle the sentinel value check and rounding:

```rust
if reserve.config.fees.flash_loan_fee_sf == u64::MAX {
    msg!("Flash loans are disabled for this reserve");
    return err!(LendingError::FlashLoansDisabled);
}
```

The check only validates against `u64::MAX` but doesn't ensure the fee is non-zero. Combined with rounding in fee calculations, attackers could potentially execute flash loans with minimal fees.

**Exploit Scenario:**
1. Find reserves with very low flash loan fees
2. Borrow amounts that cause fee calculation to round down to 0
3. Execute profitable arbitrage with zero-fee flash loans

**Recommended Fix:**
- Add minimum fee validation
- Ensure fee calculations round up, not down
- Validate total repayment amount includes fees

### 4. **[HIGH] Liquidation Bonus Manipulation**

**Severity:** High  
**Affected Files:** `state/liquidation_operations.rs:79-177`  
**Impact:** Over-redemption during liquidations

**Description:**
The liquidation bonus calculation doesn't properly validate bounds in all paths:

```rust
let bonus_multiplier = liquidation_bonus_rate + Fraction::ONE;
let total_liquidation_value_including_bonus = 
    borrowed_value * liquidation_ratio * bonus_multiplier;
```

When combined with edge cases in collateral value calculations and rounding, liquidators could receive more collateral than intended.

**Exploit Scenario:**
1. Create positions with specific debt/collateral ratios
2. Trigger liquidation with carefully calculated amounts
3. Exploit rounding in bonus calculation to receive excess collateral

**Recommended Fix:**
- Add explicit bonus bounds validation
- Ensure conservative rounding in liquidator's favor
- Add post-liquidation invariant checks

### 5. **[HIGH] Withdrawal Cap Signed Integer Vulnerability**

**Severity:** High  
**Affected Files:** `lending_market/withdrawal_cap_operations.rs:110-126`  
**Impact:** Bypass of withdrawal limits

**Description:**
The withdrawal cap uses signed integers (`i64`) for `config_capacity` and `current_total`:

```rust
if caps.config_capacity < 0 {
    return Err(LendingError::WithdrawalCapReached);
}
```

The signed arithmetic and conversion between `u64` and `i64` can lead to bypasses when values approach type boundaries.

**Exploit Scenario:**
1. Manipulate withdrawal accumulator near type boundaries
2. Cause signed/unsigned conversion issues
3. Bypass withdrawal caps to drain reserves

**Recommended Fix:**
- Use unsigned types consistently
- Add overflow checks on all conversions
- Validate cap integrity after each operation

### 6. **[MEDIUM] Collateral Exchange Rate Manipulation**

**Severity:** Medium  
**Affected Files:** `state/reserve.rs:874-978`  
**Impact:** Value extraction through exchange rate manipulation

**Description:**
The collateral exchange rate calculations have different rounding modes that could be exploited:

```rust
pub fn collateral_to_liquidity(&self, collateral_amount: u64) -> u64 {
    self.fraction_collateral_to_liquidity(collateral_amount.into())
        .to_floor()  // Rounds down
}

pub fn collateral_to_liquidity_ceil(&self, collateral_amount: u64) -> u64 {
    // Complex ceiling calculation
}
```

The inconsistent rounding between floor and ceiling operations could be exploited in deposit/withdraw sequences.

**Exploit Scenario:**
1. Repeatedly deposit and withdraw small amounts
2. Exploit rounding differences to slowly extract value
3. Accumulate extracted value over many transactions

**Recommended Fix:**
- Use consistent rounding that favors the protocol
- Add minimum operation amounts to prevent dust attacks
- Track and limit rounding losses per user

### 7. **[MEDIUM] Referral Fee Calculation Overflow**

**Severity:** Medium  
**Affected Files:** `state/reserve.rs:693-705`  
**Impact:** Incorrect fee distribution

**Description:**
The referral fee calculation doesn't validate that `absolute_referral_rate <= protocol_take_rate`:

```rust
let absolute_referral_rate = protocol_take_rate * referral_rate;
let max_referrers_fees_f = net_new_variable_debt_f * absolute_referral_rate;
```

This could lead to referrers receiving more fees than intended, depleting protocol reserves.

**Recommended Fix:**
- Add explicit validation: `assert!(absolute_referral_rate <= protocol_take_rate)`
- Ensure fee sum doesn't exceed total interest generated

### 8. **[MEDIUM] Missing PDA Bump Validation**

**Severity:** Medium  
**Affected Files:** `utils/seeds.rs`, various handlers  
**Impact:** Potential account spoofing

**Description:**
PDA derivations in several places don't validate the bump seed, relying on Anchor's validation which may not cover all cases:

```rust
#[account(
    seeds = [seeds::LENDING_MARKET_AUTH, lending_market.key().as_ref()],
    bump = lending_market.load()?.bump_seed as u8,
)]
```

**Recommended Fix:**
- Always recompute and validate PDA addresses
- Never trust user-provided bumps
- Use `find_program_address` for validation

## Additional Security Concerns

### 1. Admin Key Compromise Impact
The protocol has significant admin powers including:
- Updating reserve configurations
- Changing fee structures
- Modifying risk parameters

**Recommendation:** Implement timelocks and multi-sig requirements for critical admin operations.

### 2. Oracle Dependency Risks
The protocol relies heavily on external price oracles (Pyth, Switchboard, Scope) without sufficient fallback mechanisms.

**Recommendation:** Implement circuit breakers and multiple oracle aggregation with outlier detection.

### 3. Token-2022 Extension Validation
While the protocol includes Token-2022 validation, the coverage may not be complete for all instruction paths.

**Recommendation:** Audit all token transfer paths for proper extension validation.

## Testing Recommendations

1. **Fuzz Testing Suite**
   - Implement property-based testing for all mathematical operations
   - Test instruction sequences with randomized parameters
   - Focus on boundary conditions and type conversions

2. **Invariant Testing**
   - Total value locked should never decrease except for withdrawals
   - Sum of all debts should equal sum of all supplied liquidity minus reserves
   - Exchange rates should be monotonically increasing (absent losses)

3. **Economic Attack Simulations**
   - Flash loan attack sequences
   - Oracle manipulation scenarios
   - Liquidation cascades under extreme market conditions

## Immediate Action Items

1. **Critical**: Fix integer overflow in interest calculations
2. **Critical**: Tighten price staleness validation
3. **High**: Add minimum fee requirements for flash loans
4. **High**: Validate liquidation bonus bounds
5. **High**: Fix withdrawal cap signed integer issues
6. **Medium**: Standardize rounding modes across the protocol
7. **Medium**: Add comprehensive PDA validation

## Conclusion

The Klend protocol contains several critical vulnerabilities that could lead to significant financial losses. The most severe issues involve mathematical operations, price oracle handling, and liquidation mechanics. These vulnerabilities should be addressed immediately before any mainnet deployment or significant value is at risk.

The protocol would benefit from:
1. Comprehensive fuzzing and formal verification of mathematical operations
2. Stricter validation of external inputs and oracle data
3. Consistent rounding strategies that favor protocol safety
4. Enhanced admin controls with timelocks and multi-sig requirements

All identified vulnerabilities should be patched, and the fixes should be validated through thorough testing including unit tests, integration tests, and economic simulations.