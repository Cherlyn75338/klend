### 17 — Lending Checks Deep Dive (`lending_market/lending_checks.rs`)

Scope: Pre/postcondition guards for handlers, Token-2022 validation, and vault/accounting reconciliation.

Entry Checks by Instruction
- Borrow: blocks vault-as-destination, requires active version and non-obsolete status, validates Token-2022 extensions for liquidity mint and user destination.
- Deposit reserve liquidity: blocks vault-as-source and vault-as-destination for collateral, enforces version, validates Token-2022 extensions on source.
- Deposit obligation collateral: blocks vault-as-source, version/status checks.
- Repay: blocks vault-as-source, version check, Token-2022 validation on source.
- Redeem collateral: blocks vault-as-source and vault-as-destination, version check, Token-2022 validation.
- Liquidate: blocks misuse of vaults on both sides for repay/withdraw reserves; enforces versions; validates Token-2022 on both directions.
- Withdraw obligation collateral: version, blocks vault-as-destination, Token-2022 validation.
- Flash borrow/repay: standard vault misuse blocks; flash borrow denies when fee is disabled; Token-2022 validations.

Post-Transfer Accounting Invariant
- `post_transfer_vault_balance_liquidity_reserve_checks`:
  - Conserves `vault_balance - available_liquidity` across the transfer.
  - Verifies expected vault and available amounts against the action delta (additive, subtractive, or signed-subtractive for liquidation accounting cases).
  - Returns precise error codes for mismatches.

PDA/Referrer Validation
- `validate_referrer_token_state`:
  - Confirms initialized fields, correct mint, and recomputed PDA via `create_program_address` using canonical seeds (`BASE_SEED_REFERRER_TOKEN_STATE`, referrer, reserve, bump from account).
  - Ensures `owner_referrer` matches stored referrer.

Observations / Recommendations
- Strong coverage of Token-2022 extension validation at key surfaces; verify any newly added handler also applies it.
- Vault misuse blocks are consistent and prevent common account redirection bugs.
- Consider constraining sysvar `instructions` addresses explicitly on flows relying on introspection (if any are added later).

Links
- Complements invariant coverage in 10_invariants.md and instruction checklist in 05_instruction_checklist.md.


