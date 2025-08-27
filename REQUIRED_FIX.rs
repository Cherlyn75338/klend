// REQUIRED FIX FOR CRITICAL VULNERABILITY
// Add this validation to token_transfer.rs and spltoken.rs

use anchor_lang::{prelude::*, solana_program::program_error::ProgramError};
use spl_token::ID as SPL_TOKEN_ID;
use spl_token_2022::ID as SPL_TOKEN_2022_ID;

/// Validates that the provided token program is either SPL Token or Token-2022
pub fn validate_token_program(token_program: &AccountInfo) -> Result<()> {
    if token_program.key != &SPL_TOKEN_ID && token_program.key != &SPL_TOKEN_2022_ID {
        msg!("Invalid token program: {}", token_program.key);
        return err!(LendingError::InvalidTokenProgram);
    }
    Ok(())
}

// Fix for token_transfer.rs functions:

pub fn deposit_obligation_collateral_transfer<'a>(
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    collateral_amount: u64,
) -> Result<()> {
    // ADD THIS VALIDATION
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
    // ADD THESE VALIDATIONS
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

    spltoken::mint(
        collateral_token_program,
        collateral_mint,
        mint_authority,
        destination_collateral,
        authority_signer_seeds,
        collateral_mint_amount,
    )?;

    Ok(())
}

// Apply same fix to ALL other functions in token_transfer.rs

// Fix for spltoken.rs functions:

pub fn mint<'info>(
    token_program: AccountInfo<'info>,
    token_mint: AccountInfo<'info>,
    token_mint_authority: AccountInfo<'info>,
    user_token_ata: AccountInfo<'info>,
    authority_signer_seeds: &[&[u8]],
    mint_amount: u64,
) -> Result<()> {
    // ADD THIS VALIDATION
    if token_program.key != &SPL_TOKEN_ID {
        msg!("Invalid token program for mint: {}", token_program.key);
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
    // ADD THIS VALIDATION
    if token_program.key != &SPL_TOKEN_ID {
        msg!("Invalid token program for burn: {}", token_program.key);
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

// Add new error variant to LendingError enum:
#[error_code]
pub enum LendingError {
    // ... existing errors ...
    
    #[msg("Invalid token program. Must be SPL Token or Token-2022")]
    InvalidTokenProgram,
}

// Test to verify the fix works:
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reject_malicious_token_program() {
        let malicious_program = Pubkey::new_unique();
        let account_info = AccountInfo::new(
            &malicious_program,
            false,
            true,  // executable
            &mut [],
            &mut [],
            &system_program::ID,
            false,
            0,
        );
        
        let result = validate_token_program(&account_info);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            LendingError::InvalidTokenProgram.into()
        );
    }
    
    #[test]
    fn test_accept_valid_token_programs() {
        // Test SPL Token
        let spl_token_info = AccountInfo::new(
            &SPL_TOKEN_ID,
            false,
            true,
            &mut [],
            &mut [],
            &system_program::ID,
            false,
            0,
        );
        assert!(validate_token_program(&spl_token_info).is_ok());
        
        // Test Token-2022
        let token_2022_info = AccountInfo::new(
            &SPL_TOKEN_2022_ID,
            false,
            true,
            &mut [],
            &mut [],
            &system_program::ID,
            false,
            0,
        );
        assert!(validate_token_program(&token_2022_info).is_ok());
    }
}