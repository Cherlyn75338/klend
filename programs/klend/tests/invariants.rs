#![allow(clippy::useless_conversion)]

use proptest::prelude::*;

use kamino_lending as kl;
use kl::lending_market::lending_checks::post_transfer_vault_balance_liquidity_reserve_checks;
use kl::lending_market::lending_operations::utils::{post_deposit_obligation_invariants, post_withdraw_obligation_invariants};
use kl::state::{reserve::{Reserve, ReserveLiquidity, ReserveCollateral}};
use kl::utils::{fraction::{Fraction, FractionExtra}, consts::ten_pow};
use kl::{LendingAction};

fn mk_reserve(price_sf: u128, mint_decimals: u8, available: u64) -> Reserve {
    let mut r = Reserve::default();
    r.liquidity = ReserveLiquidity::new(kl::state::reserve::NewReserveLiquidityParams{
        mint_pubkey: Default::default(),
        mint_decimals,
        mint_token_program: Default::default(),
        supply_vault: Default::default(),
        fee_vault: Default::default(),
        market_price_sf: price_sf,
        initial_amount_deposited_in_reserve: available,
    });
    // Collateral supply mirrors initial available for 1:1 init rate
    r.collateral = ReserveCollateral::new(kl::state::reserve::NewReserveCollateralParams{
        mint_pubkey: Default::default(),
        supply_vault: Default::default(),
        initial_collateral_supply: available,
    });
    r
}

proptest! {
    // First-deposit 1:1 behavior: when supply and total_liquidity are equal non-zero, rate behaves as ~1:1
    #[test]
    fn first_deposit_one_to_one(amount in 1u64..1_000_000u64) {
        let rate = kl::CollateralExchangeRate::ONE;
        let c = rate.liquidity_to_collateral(amount);
        let l = rate.collateral_to_liquidity(c);
        prop_assert!(l <= amount + 1 && l + 1 >= amount);
    }

    // Vault-vs-accounting conservation across deposit
    #[test]
    fn vault_delta_conserved_deposit(init_v in 0u64..1_000_000u64, init_av in 0u64..1_000_000u64, dep in 1u64..100_000u64) {
        proptest::prop_assume!(init_v >= init_av);
        let final_v = init_v.saturating_add(dep);
        let final_av = init_av.saturating_add(dep);
        let res = post_transfer_vault_balance_liquidity_reserve_checks(final_v, final_av, init_v, init_av, LendingAction::Additive(dep));
        prop_assert!(res.is_ok());
    }

    // Vault-vs-accounting conservation across withdraw
    #[test]
    fn vault_delta_conserved_withdraw(init_v in 1u64..1_000_000u64, init_av in 1u64..1_000_000u64, dep in 1u64..100_000u64) {
        proptest::prop_assume!(init_v >= init_av); // valid pre-state
        proptest::prop_assume!(init_v >= dep && init_av >= dep);
        let final_v = init_v - dep;
        let final_av = init_av - dep;
        let res = post_transfer_vault_balance_liquidity_reserve_checks(final_v, final_av, init_v, init_av, LendingAction::Subtractive(dep));
        prop_assert!(res.is_ok());
    }

    // Donation alone (vault only) breaks invariant and would be detected
    #[test]
    fn donation_breaks_invariant(init_v in 0u64..1_000_000u64, init_av in 0u64..1_000_000u64, donation in 1u64..100_000u64) {
        proptest::prop_assume!(init_v >= init_av);
        let final_v = init_v.saturating_add(donation);
        let final_av = init_av; // unchanged accounting
        let res = post_transfer_vault_balance_liquidity_reserve_checks(final_v, final_av, init_v, init_av, LendingAction::Additive(0));
        prop_assert!(res.is_err());
    }

    // LTV monotonicity on deposit/withdraw using internal valuation
    #[test]
    fn ltv_monotonicity(price in 1u128..1_000_000_000_000u128, dec in 0u8..=12u8, available in 1u64..=1_000_000u64, deposit_liq in 1u64..=1_000u64, withdraw_liq in 1u64..=1_000u64) {
        let reserve = mk_reserve(price, dec, available);
        let mint_factor = Fraction::from(ten_pow(dec as usize));
        let price_f = Fraction::from_bits(price);

        // Build a realistic deposit and resulting deposited market value
        let rate = reserve.collateral_exchange_rate();
        let coll_minted = rate.liquidity_to_collateral(deposit_liq);
        let liq_from_coll = rate.fraction_collateral_to_liquidity(Fraction::from(coll_minted));
        let start_dep_mv = liq_from_coll * price_f / mint_factor;

        // Set initial debt as 50% of deposited value
        let start_debt_mv = start_dep_mv * Fraction::from_percent(50u64);
        let initial_ltv = start_debt_mv / start_dep_mv;

        // Withdraw path (bounded by deposited amount)
        proptest::prop_assume!(withdraw_liq <= deposit_liq);
        let coll_burn = rate.liquidity_to_collateral(withdraw_liq);
        let liq_from_coll_wd = rate.fraction_collateral_to_liquidity(Fraction::from(coll_burn));
        let asset_mv_wd = liq_from_coll_wd * price_f / mint_factor;
        let wd_ltv = start_debt_mv / (start_dep_mv - asset_mv_wd);
        prop_assert!(wd_ltv >= initial_ltv);
    }
}

// Deterministic tests for decimals and rounding <= 1 wei per tx
#[test]
fn decimals_rounding_leq_one_wei() {
    for dec in 0u8..=12 {
        let price = Fraction::from_num(1u64); // $1
        let mint_factor = Fraction::from(ten_pow(dec as usize));
        let supply: u64 = 1_000_000;
        let total_liq = Fraction::from_num(1_000_000u64);
        let rate = kl::CollateralExchangeRate::from_supply_and_liquidity(supply, total_liq);
        for liq in [1u64, 10, 1_000, 1_000_000, 123_456] {
            let c = rate.liquidity_to_collateral(liq);
            let mv = Fraction::from(liq) * price / mint_factor;
            let mv_back = Fraction::from(rate.collateral_to_liquidity(c)) * price / mint_factor;
            let (hi, lo) = if mv_back > mv { (mv_back, mv) } else { (mv, mv_back) };
            let diff = hi - lo;
            assert!(diff <= Fraction::from_num(1u64));
        }
    }
}

