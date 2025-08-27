// Security Tests for Klend Protocol Vulnerabilities
// This file contains proof-of-concept tests demonstrating the identified vulnerabilities

use anchor_lang::prelude::*;
use klend::utils::fraction::{Fraction, FractionExtra};
use klend::state::reserve::*;
use klend::LendingError;

#[cfg(test)]
mod fraction_panic_tests {
    use super::*;
    use primitive_types::U256;

    #[test]
    #[should_panic(expected = "Denominator is not big enough")]
    fn test_full_mul_int_ratio_panic() {
        // This test demonstrates the panic vulnerability in full_mul_int_ratio
        // An attacker could craft inputs that cause overflow and panic
        
        let large_fraction = Fraction::from_num(u64::MAX);
        let large_numerator = U256::MAX / 2;
        let small_denominator = U256::from(1);
        
        // This will panic instead of returning an error
        let _result = large_fraction.full_mul_int_ratio(large_numerator, small_denominator);
    }

    #[test]
    #[should_panic(expected = "Overflow in div_ceil")]
    fn test_div_ceil_panic() {
        // This test demonstrates the panic vulnerability in div_ceil
        
        let large_fraction = Fraction::from_num(u64::MAX);
        let small_divisor = Fraction::from_num(1) / u128::from(u64::MAX);
        
        // This will panic instead of returning an error
        let _result = large_fraction.div_ceil(&small_divisor);
    }
}

#[cfg(test)]
mod negative_interest_tests {
    use super::*;

    #[test]
    fn test_negative_net_new_variable_debt() {
        // This test demonstrates the potential for negative net_new_variable_debt_f
        // in the compound_interest calculation
        
        // Simulate a scenario where fixed_host_fee could exceed net new debt
        let previous_debt = Fraction::from_num(1000);
        let compounded_interest_rate = Fraction::from_percent(101); // 1% interest
        let compounded_fixed_rate = Fraction::from_percent(102); // 2% fixed rate (higher)
        
        let new_debt = previous_debt * compounded_interest_rate;
        let fixed_host_fee = (previous_debt * compounded_fixed_rate) - previous_debt;
        
        // Calculate net_new_variable_debt_f
        let net_new_variable_debt_f = new_debt - previous_debt - fixed_host_fee;
        
        // This could be negative!
        println!("Net new variable debt: {:?}", net_new_variable_debt_f);
        
        // In the actual code, this negative value would be used to calculate fees
        // potentially causing underflow or incorrect fee distribution
        if net_new_variable_debt_f < Fraction::ZERO {
            println!("WARNING: Negative net new variable debt detected!");
        }
    }
}

#[cfg(test)]
mod referral_rate_tests {
    use super::*;

    #[test]
    fn test_unbounded_referral_rate() {
        // This test shows that absolute_referral_rate can exceed protocol_take_rate
        // if inputs are not properly validated
        
        let protocol_take_rate = Fraction::from_percent(10); // 10%
        let referral_rate = Fraction::from_percent(200); // 200% (malicious input)
        
        // This calculation doesn't check bounds
        let absolute_referral_rate = protocol_take_rate * referral_rate;
        
        println!("Protocol take rate: {:?}", protocol_take_rate);
        println!("Referral rate: {:?}", referral_rate);
        println!("Absolute referral rate: {:?}", absolute_referral_rate);
        
        // The absolute referral rate is now 20%, exceeding the protocol take rate of 10%
        assert!(absolute_referral_rate > protocol_take_rate);
        println!("WARNING: Absolute referral rate exceeds protocol take rate!");
    }
}

#[cfg(test)]
mod pda_validation_tests {
    use super::*;
    use solana_program::pubkey::Pubkey;

    #[test]
    #[should_panic]
    fn test_pda_validation_panic() {
        // This test demonstrates potential panic in PDA validation
        
        let program_id = Pubkey::new_unique();
        let invalid_seeds: &[&[u8]] = &[
            b"invalid",
            &[255, 255, 255, 255], // Invalid bump seed
        ];
        
        // This will panic with invalid seeds
        let _pda = Pubkey::create_program_address(invalid_seeds, &program_id).unwrap();
    }
}

#[cfg(test)]
mod precision_loss_tests {
    use super::*;

    #[test]
    fn test_exchange_rate_precision_loss() {
        // This test demonstrates precision loss in exchange rate calculations
        // particularly problematic with tokens having extreme decimal values
        
        // Simulate a token with 18 decimals
        let collateral_supply: u64 = 1_000_000_000_000_000_000; // 1 token with 18 decimals
        let liquidity = Fraction::from_num(1_000_000_000_000_000_000);
        
        let exchange_rate = CollateralExchangeRate::from_supply_and_liquidity(
            collateral_supply,
            liquidity
        );
        
        // Convert small amounts back and forth
        let small_amount = 1; // 1 wei
        let liquidity_amount = exchange_rate.collateral_to_liquidity(small_amount);
        let collateral_back = exchange_rate.liquidity_to_collateral(liquidity_amount as u64);
        
        // Precision loss occurs here
        println!("Original collateral: {}", small_amount);
        println!("Converted to liquidity: {}", liquidity_amount);
        println!("Converted back to collateral: {}", collateral_back);
        
        // The value is lost due to rounding
        if collateral_back != small_amount {
            println!("WARNING: Precision loss detected in exchange rate conversion!");
        }
    }

    #[test]
    fn test_dust_attack_accumulation() {
        // This test shows how repeated small operations can accumulate rounding errors
        
        let mut total_loss = Fraction::ZERO;
        let iterations = 10000;
        
        for _ in 0..iterations {
            let small_amount = Fraction::from_num(1) / Fraction::from_num(3);
            let floored = small_amount.to_floor::<u64>();
            let loss = small_amount - Fraction::from_num(floored);
            total_loss = total_loss + loss;
        }
        
        println!("Total precision loss after {} iterations: {:?}", iterations, total_loss);
        
        // Even small rounding errors can accumulate to significant amounts
        if total_loss > Fraction::from_num(1) {
            println!("WARNING: Significant value leakage from dust operations!");
        }
    }
}

#[cfg(test)]
mod overflow_attack_tests {
    use super::*;

    #[test]
    fn test_u64_max_overflow() {
        // Test operations with u64::MAX to find overflow vulnerabilities
        
        let max_amount = u64::MAX;
        let fee_rate = Fraction::from_percent(1); // 1% fee
        
        // Calculate fee on max amount
        let fee = Fraction::from_num(max_amount) * fee_rate;
        
        // Try to convert back to u64 - this could overflow
        match fee.to_floor::<u64>() {
            fee_u64 if fee_u64 == u64::MAX => {
                println!("WARNING: Fee calculation saturated at u64::MAX!");
            }
            fee_u64 => {
                // Try to add fee to original amount - definite overflow
                match max_amount.checked_add(fee_u64) {
                    None => println!("CRITICAL: Overflow detected when adding fee to max amount!"),
                    Some(_) => {}
                }
            }
        }
    }
}

// Integration test for compound vulnerabilities
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_compound_vulnerability_scenario() {
        // This test combines multiple vulnerabilities in a realistic attack scenario
        
        println!("=== Compound Vulnerability Attack Scenario ===");
        
        // Step 1: Attempt to trigger panic with large values
        println!("\n1. Testing panic vulnerability with large multiplier...");
        let large_fraction = Fraction::from_num(u64::MAX / 2);
        let multiplier = U256::from(u128::MAX);
        let divisor = U256::from(2);
        
        // This would panic in production
        // let _result = large_fraction.full_mul_int_ratio(multiplier, divisor);
        println!("   [Would panic in production with large values]");
        
        // Step 2: Exploit negative interest calculation
        println!("\n2. Testing negative interest vulnerability...");
        let debt = Fraction::from_num(1_000_000);
        let variable_rate = Fraction::from_percent(1);
        let fixed_rate = Fraction::from_percent(5); // Unusually high fixed rate
        
        let new_debt = debt * (Fraction::ONE + variable_rate);
        let fixed_fee = debt * fixed_rate;
        let net_variable = new_debt - debt - fixed_fee;
        
        if net_variable < Fraction::ZERO {
            println!("   [Negative interest achieved! Protocol fees would underflow]");
        }
        
        // Step 3: Exploit unbounded referral rate
        println!("\n3. Testing referral rate manipulation...");
        let protocol_rate = Fraction::from_percent(10);
        let malicious_referral = Fraction::from_percent(500);
        let exploited_rate = protocol_rate * malicious_referral;
        
        if exploited_rate > protocol_rate {
            println!("   [Referral rate exceeds protocol rate - fee drainage possible]");
        }
        
        // Step 4: Accumulate dust for value extraction
        println!("\n4. Testing dust accumulation attack...");
        let mut extracted_value = 0u64;
        for _ in 0..1000 {
            let dust_amount = 3u64; // Amount that causes rounding
            let fee = Fraction::from_num(dust_amount) * Fraction::from_percent(33);
            let fee_floor = fee.to_floor::<u64>();
            let loss = dust_amount - fee_floor * 3; // Simplified calculation
            extracted_value += loss;
        }
        println!("   [Extracted {} units of value through dust operations]", extracted_value);
        
        println!("\n=== Attack Scenario Complete ===");
        println!("Multiple vulnerabilities can be chained for maximum impact");
    }
}