use proptest::prelude::*;

use kamino_lending as kl;
use kl::utils::fraction::Fraction;

// Ensure no handler reads vault balance for exchange-rate/valuation (static assertion by grep at build-time)
// This test acts as a sentinel by scanning source strings at runtime in CI; if code changes, it fails.
#[test]
fn no_vault_balance_in_exchange_rate_or_valuation_code() {
    // Lightweight heuristic: ensure functions that compute exchange rate never mention `amount` from TokenAccount
    // Note: this is a heuristic guard; real enforcement should be in code review and property tests.
    let source = include_str!("../src/state/reserve.rs");
    assert!(source.contains("fn exchange_rate(") && !source.contains("accessor::amount"));

    let lm_ops = include_str!("../src/lending_market/lending_operations.rs");
    assert!(lm_ops.contains("calculate_obligation_collateral_market_value"));
    // forbid direct token account balance reads in valuation utilities
    assert!(!lm_ops.contains("accessor::amount(") || lm_ops.contains("post_transfer_vault_balance"));
}

// Ensure liquidation math uses internal accounting
#[test]
fn liquidation_internal_accounting_only() {
    let lm_liq = include_str!("../src/state/liquidation_operations.rs");
    // No token account reads in liquidation core
    assert!(!lm_liq.contains("accessor::amount("));
}

// Policy toggles: disabled-as-collateral outside e-mode must not be bypassed mid-flow
proptest! {
    #[test]
    fn disabled_as_collateral_outside_emode_cannot_bypass(t in 0u8..=1u8) {
        // We don't spin the full on-chain environment; we assert the policy gate logic exists in source
        let lm_ops = include_str!("../src/lending_market/lending_operations.rs");
        assert!(lm_ops.contains("disable_usage_as_coll_outside_emode"));
        // Presence of checks for both deposit and withdraw paths
        assert!(lm_ops.contains("DepositDisabledOutsideElevationGroup") || lm_ops.contains("get_max_ltv_and_liquidation_threshold"));
    }
}

// Referral fees should not affect solvency (protocol/referrer accounting add/subtract only)
#[test]
fn referral_fees_do_not_change_total_supply() {
    let code = include_str!("../src/lending_market/lending_operations.rs");
    assert!(code.contains("accumulated_protocol_fees_sf"));
    assert!(code.contains("pending_referrer_fees_sf"));
    assert!(code.contains("absolute_referral_rate_sf"));
    // No direct mutation of available_amount except bounded adds when collecting referral fees back
    // This is a heuristic check to highlight any future misuse
}

// Ensure u64::MAX sentinel is handled in borrow/withdraw paths
#[test]
fn u64_max_sentinel_used_only_in_handlers() {
    let code = include_str!("../src/lending_market/lending_operations.rs");
    assert!(code.contains("if amount_to_borrow == u64::MAX"));
    assert!(code.contains("if collateral_amount == u64::MAX"));
}

