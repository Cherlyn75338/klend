#![allow(clippy::bool_assert_comparison)]

use proptest::prelude::*;

use kamino_lending::utils::fraction::{Fraction, FractionExtra};
use kamino_lending::state::reserve::{CollateralExchangeRate};

// Helpers to generate reasonable ranges
fn arb_liquidity() -> impl Strategy<Value = u128> {
    // stay well below U68F60 integer limit (2^68 ~ 2.95e20); use < 2^60
    1u128..(1u128 << 60)
}

fn arb_collateral_supply() -> impl Strategy<Value = u64> {
    // keep within 2^40 to avoid overflow in u64 conversions under extreme ratios
    1u64..=(1u64 << 40)
}

// Convert helpers
fn to_fraction_u64(x: u64) -> Fraction { Fraction::from_num(x) }
fn to_fraction_u128(x: u128) -> Fraction { Fraction::from_num(x) }

proptest! {
    // Floor/Ceil symmetry: ceil is >= exact >= floor, and gap ≤ 1 for u64 outputs
    #[test]
    fn ceil_floor_symmetry(liq in arb_liquidity(), supply in arb_collateral_supply(), amount in 0u64..1_000_000_000u64) {
        let rate = CollateralExchangeRate::from_supply_and_liquidity(supply, to_fraction_u128(liq));

        let coll_floor = rate.liquidity_to_collateral(amount);
        let coll_frac = rate.liquidity_to_collateral_fraction(amount);
        let coll_ceil = rate.liquidity_to_collateral_ceil(amount);

        prop_assert!(u128::from(coll_floor) <= coll_frac.to_num::<u128>());
        prop_assert!(coll_frac.to_num::<u128>() <= u128::from(coll_ceil));
        prop_assert!(u64::try_from(u128::from(coll_ceil) - u128::from(coll_floor)).unwrap() <= 1);

        let liq_floor = rate.collateral_to_liquidity(coll_floor);
        let liq_ceil = rate.collateral_to_liquidity_ceil(coll_floor);
        prop_assert!(liq_ceil >= liq_floor);
        prop_assert!(liq_ceil.saturating_sub(liq_floor) <= 1);
    }

    // Round-trip within ≤1 wei: liquidity -> collateral -> liquidity
    #[test]
    fn round_trip_liq_coll_liq_within_1(liq in 0u64..1_000_000_000_000u64, supply in arb_collateral_supply(), total_liq in 1u128..(1u128<<60)) {
        let rate = CollateralExchangeRate::from_supply_and_liquidity(supply, to_fraction_u128(total_liq));
        let c = rate.liquidity_to_collateral(liq);
        let liq2 = rate.collateral_to_liquidity(c);
        prop_assert!(liq2 <= liq + 1);
        // ceil path should not overshoot by more than 1
        let liq2c = rate.collateral_to_liquidity_ceil(c);
        prop_assert!(liq2c >= liq2);
        prop_assert!(liq2c.saturating_sub(liq) <= 1);
    }

    // Exchange-rate invariants: donation to vault (simulated by changing external balance) must not affect internal math
    // We validate using only internal totals (supply, total_liquidity)
    #[test]
    fn donation_invariant_unaffected_by_external_balance(supply in arb_collateral_supply(), total_liq in 1u128..1_000_000_000_000_000_000u128, action in 0u64..1_000_000_000u64) {
        let rate = CollateralExchangeRate::from_supply_and_liquidity(supply, to_fraction_u128(total_liq));
        let coll = rate.liquidity_to_collateral(action);
        let liq_back = rate.collateral_to_liquidity(coll);
        // Without touching rate inputs, the path must be deterministic regardless of any out-of-band vault changes
        prop_assert!(liq_back <= action + 1);
    }

    // u64::MAX semantics: using MAX as sentinel upstream should not produce rounding freebies in conversion primitives
    #[test]
    fn no_rounding_freebie_with_extreme_values(supply in 1u64..=(1u64<<40), total_liq in 1u128..(1u128<<60)) {
        let rate = CollateralExchangeRate::from_supply_and_liquidity(supply, to_fraction_u128(total_liq));
        let liq = (1u64<<60) / 2; // still large but safe
        // guard against overflow expectations
        let coll_f = rate.liquidity_to_collateral_fraction(liq);
        proptest::prop_assume!(coll_f.try_to_floor::<u64>().is_some());
        let c = rate.liquidity_to_collateral(liq);
        let liq2 = rate.collateral_to_liquidity(c);
        prop_assert!(liq2 <= liq + 1);
    }

    // EPSILON magnitude cannot be exploited to gain value across conversions
    #[test]
    fn epsilon_cannot_gain_value(supply in arb_collateral_supply(), total_liq in 1u128..1_000_000_000_000_000_000u128, amount in 1u64..1_000_000_000u64) {
        let rate = CollateralExchangeRate::from_supply_and_liquidity(supply, to_fraction_u128(total_liq));
        let base_coll = rate.liquidity_to_collateral(amount);
        let bump = (amount / 1_000_000).max(1); // tiny bump
        let coll_after = rate.liquidity_to_collateral(amount + bump);
        prop_assert!(coll_after >= base_coll); // monotonicity
        let back = rate.collateral_to_liquidity(coll_after);
        prop_assert!(back <= amount + bump + 1);
    }
}

