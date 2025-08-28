// CRITICAL SECURITY FIX: Token Program Validation
// This file contains the required changes to fix the vulnerability

use anchor_lang::{prelude::*, solana_program::program_error::ProgramError};
use spl_token::ID as SPL_TOKEN_ID;
use spl_token_2022::ID as SPL_TOKEN_2022_ID;

// ============================================================================
// STEP 1: Add validation function to utils module
// ============================================================================

/// Validates that the provided token program is either SPL Token or Token-2022
/// CRITICAL: This function MUST be called before any CPI to a token program
pub fn validate_token_program(token_program: &AccountInfo) -> Result<()> {
    if token_program.key != &SPL_TOKEN_ID && token_program.key != &SPL_TOKEN_2022_ID {
        msg!("ERROR: Invalid token program detected!");
        msg!("  Provided: {}", token_program.key);
        msg!("  Expected: {} or {}", SPL_TOKEN_ID, SPL_TOKEN_2022_ID);
        return err!(LendingError::InvalidTokenProgram);
    }
    Ok(())
}

// ============================================================================
// STEP 2: Fix all functions in token_transfer.rs
// ============================================================================

pub mod fixed_token_transfer {
    use super::*;
    use anchor_spl::token_interface;

    pub fn deposit_obligation_collateral_transfer<'a>(
        from: AccountInfo<'a>,
        to: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        collateral_amount: u64,
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program
        validate_token_program(&token_program)?;

        #[allow(deprecated)]
        token_interface::transfer(
            CpiContext::new(
                token_program,
                token_interface::Transfer {
                    from,
                    to,
                    authority,
                },
            ),
            collateral_amount,
        )?;
        Ok(())
    }

    pub fn deposit_reserve_liquidity_transfer<'a>(
        source_liquidity_deposit: AccountInfo<'a>,
        destination_liquidity_deposit: AccountInfo<'a>,
        user_authority: AccountInfo<'a>,
        liquidity_mint: AccountInfo<'a>,
        liquidity_token_program: AccountInfo<'a>,
        collateral_mint: AccountInfo<'a>,
        collateral_token_program: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        mint_authority: AccountInfo<'a>,
        authority_signer_seeds: &[&[u8]],
        liquidity_deposit_amount: u64,
        liquidity_decimals: u8,
        collateral_mint_amount: u64,
    ) -> Result<()> {
        // CRITICAL FIX: Validate both token programs
        validate_token_program(&liquidity_token_program)?;
        validate_token_program(&collateral_token_program)?;

        token_interface::transfer_checked(
            CpiContext::new(
                liquidity_token_program.clone(),
                token_interface::TransferChecked {
                    from: source_liquidity_deposit,
                    to: destination_liquidity_deposit,
                    authority: user_authority,
                    mint: liquidity_mint,
                },
            ),
            liquidity_deposit_amount,
            liquidity_decimals,
        )?;

        // Call fixed spltoken::mint
        fixed_spltoken::mint(
            collateral_token_program,
            collateral_mint,
            mint_authority,
            destination_collateral,
            authority_signer_seeds,
            collateral_mint_amount,
        )?;

        Ok(())
    }

    pub fn withdraw_obligation_collateral_transfer<'a>(
        token_program: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        lending_market_authority: AccountInfo<'a>,
        authority_signer_seeds: &[&[u8]],
        withdraw_amount: u64,
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program
        validate_token_program(&token_program)?;

        #[allow(deprecated)]
        token_interface::transfer(
            CpiContext::new_with_signer(
                token_program,
                token_interface::Transfer {
                    to: destination_collateral,
                    from: source_collateral,
                    authority: lending_market_authority,
                },
                &[authority_signer_seeds],
            ),
            withdraw_amount,
        )?;
        Ok(())
    }

    pub fn repay_obligation_liquidity_transfer<'a>(
        token_program: AccountInfo<'a>,
        liquidity_mint: AccountInfo<'a>,
        user_liquidity: AccountInfo<'a>,
        reserve_liquidity: AccountInfo<'a>,
        user_authority: AccountInfo<'a>,
        repay_amount: u64,
        decimals: u8,
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program
        validate_token_program(&token_program)?;

        token_interface::transfer_checked(
            CpiContext::new(
                token_program,
                token_interface::TransferChecked {
                    from: user_liquidity,
                    to: reserve_liquidity,
                    authority: user_authority,
                    mint: liquidity_mint,
                },
            ),
            repay_amount,
            decimals,
        )?;
        Ok(())
    }

    pub fn borrow_obligation_liquidity_transfer<'a>(
        token_program: AccountInfo<'a>,
        liquidity_mint: AccountInfo<'a>,
        reserve_liquidity: AccountInfo<'a>,
        user_liquidity: AccountInfo<'a>,
        lending_market_authority: AccountInfo<'a>,
        authority_signer_seeds: &[&[u8]],
        liquidity_amount: u64,
        liquidity_decimals: u8,
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program
        validate_token_program(&token_program)?;

        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                token_program,
                token_interface::TransferChecked {
                    from: reserve_liquidity,
                    to: user_liquidity,
                    authority: lending_market_authority,
                    mint: liquidity_mint,
                },
                &[authority_signer_seeds],
            ),
            liquidity_amount,
            liquidity_decimals,
        )?;
        Ok(())
    }

    // Apply same fix to ALL other functions:
    // - deposit_initial_reserve_liquidity_transfer
    // - deposit_reserve_liquidity_and_obligation_collateral_transfer
    // - redeem_reserve_collateral_transfer
    // - withdraw_and_redeem_reserve_collateral_transfer
    // - pay_borrowing_fees_transfer
    // - send_origination_fees_transfer
    // - withdraw_fees_from_reserve
}

// ============================================================================
// STEP 3: Fix all functions in spltoken.rs
// ============================================================================

pub mod fixed_spltoken {
    use super::*;

    pub fn mint<'info>(
        token_program: AccountInfo<'info>,
        token_mint: AccountInfo<'info>,
        token_mint_authority: AccountInfo<'info>,
        user_token_ata: AccountInfo<'info>,
        authority_signer_seeds: &[&[u8]],
        mint_amount: u64,
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program (only SPL Token for mint)
        if token_program.key != &SPL_TOKEN_ID {
            msg!("ERROR: Invalid token program for mint operation");
            msg!("  Provided: {}", token_program.key);
            msg!("  Expected: {}", SPL_TOKEN_ID);
            return err!(LendingError::InvalidTokenProgram);
        }

        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                token_program,
                anchor_spl::token::MintTo {
                    mint: token_mint,
                    to: user_token_ata,
                    authority: token_mint_authority,
                },
                &[authority_signer_seeds],
            ),
            mint_amount,
        )?;
        Ok(())
    }

    pub fn burn<'info>(
        token_mint: AccountInfo<'info>,
        user_token_ata: AccountInfo<'info>,
        user: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        burn_amount: u64,
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program (only SPL Token for burn)
        if token_program.key != &SPL_TOKEN_ID {
            msg!("ERROR: Invalid token program for burn operation");
            msg!("  Provided: {}", token_program.key);
            msg!("  Expected: {}", SPL_TOKEN_ID);
            return err!(LendingError::InvalidTokenProgram);
        }

        anchor_spl::token::burn(
            CpiContext::new(
                token_program,
                anchor_spl::token::Burn {
                    mint: token_mint,
                    from: user_token_ata,
                    authority: user,
                },
            ),
            burn_amount,
        )?;
        Ok(())
    }

    pub fn burn_with_signer<'info>(
        token_mint: AccountInfo<'info>,
        token_ata: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        burn_amount: u64,
        authority_signer_seeds: &[&[&[u8]]],
    ) -> Result<()> {
        // CRITICAL FIX: Validate token program (only SPL Token for burn)
        if token_program.key != &SPL_TOKEN_ID {
            msg!("ERROR: Invalid token program for burn_with_signer operation");
            msg!("  Provided: {}", token_program.key);
            msg!("  Expected: {}", SPL_TOKEN_ID);
            return err!(LendingError::InvalidTokenProgram);
        }

        anchor_spl::token::burn(
            CpiContext::new_with_signer(
                token_program,
                anchor_spl::token::Burn {
                    mint: token_mint,
                    from: token_ata,
                    authority,
                },
                authority_signer_seeds,
            ),
            burn_amount,
        )?;
        Ok(())
    }
}

// ============================================================================
// STEP 4: Add new error variant to LendingError enum
// ============================================================================

#[error_code]
pub enum LendingError {
    // ... existing errors ...

    #[msg("Invalid token program. Must be SPL Token or Token-2022")]
    InvalidTokenProgram,
}

// ============================================================================
// STEP 5: Add comprehensive tests
// ============================================================================

#[cfg(test)]
mod security_tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_reject_malicious_token_program() {
        // Create a malicious program pubkey
        let malicious_program = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = vec![];
        let owner = Pubkey::new_unique();
        
        let account_info = AccountInfo::new(
            &malicious_program,
            false,
            true,  // executable
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );

        // Should reject the malicious program
        let result = validate_token_program(&account_info);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            LendingError::InvalidTokenProgram.into()
        );
    }

    #[test]
    fn test_accept_valid_spl_token() {
        let mut lamports = 0;
        let mut data = vec![];
        let owner = Pubkey::new_unique();
        
        let spl_token_info = AccountInfo::new(
            &SPL_TOKEN_ID,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );

        // Should accept SPL Token
        assert!(validate_token_program(&spl_token_info).is_ok());
    }

    #[test]
    fn test_accept_valid_token_2022() {
        let mut lamports = 0;
        let mut data = vec![];
        let owner = Pubkey::new_unique();
        
        let token_2022_info = AccountInfo::new(
            &SPL_TOKEN_2022_ID,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );

        // Should accept Token-2022
        assert!(validate_token_program(&token_2022_info).is_ok());
    }

    #[test]
    fn test_reject_random_executable_program() {
        // Test multiple random programs
        for _ in 0..10 {
            let random_program = Pubkey::new_unique();
            let mut lamports = 0;
            let mut data = vec![];
            let owner = Pubkey::new_unique();
            
            let account_info = AccountInfo::new(
                &random_program,
                false,
                true,  // executable
                &mut lamports,
                &mut data,
                &owner,
                false,
                0,
            );

            // Should reject all random programs
            let result = validate_token_program(&account_info);
            assert!(result.is_err());
        }
    }
}

// ============================================================================
// DEPLOYMENT CHECKLIST
// ============================================================================

/*
CRITICAL SECURITY FIX DEPLOYMENT CHECKLIST:

1. [ ] Add validate_token_program() function to utils module
2. [ ] Add InvalidTokenProgram error to LendingError enum
3. [ ] Apply validation to ALL 12 functions in token_transfer.rs
4. [ ] Apply validation to ALL 3 functions in spltoken.rs
5. [ ] Run all security tests to verify fix
6. [ ] Deploy to devnet and test with actual malicious program
7. [ ] Audit the changes with security team
8. [ ] Deploy to mainnet ONLY after full verification

FUNCTIONS REQUIRING FIX:

token_transfer.rs:
- [x] deposit_obligation_collateral_transfer
- [x] deposit_reserve_liquidity_transfer
- [ ] deposit_initial_reserve_liquidity_transfer
- [ ] deposit_reserve_liquidity_and_obligation_collateral_transfer
- [x] withdraw_obligation_collateral_transfer
- [ ] redeem_reserve_collateral_transfer
- [ ] withdraw_and_redeem_reserve_collateral_transfer
- [x] repay_obligation_liquidity_transfer
- [x] borrow_obligation_liquidity_transfer
- [ ] pay_borrowing_fees_transfer
- [ ] send_origination_fees_transfer
- [ ] withdraw_fees_from_reserve

spltoken.rs:
- [x] mint
- [x] burn
- [x] burn_with_signer

TOTAL: 15 functions must be fixed

⚠️ WARNING: Do NOT deploy to mainnet until ALL functions are fixed!
*/