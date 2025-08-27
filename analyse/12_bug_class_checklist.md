### 12 — Standard Solana Bug-Class Checklist (Applied to Klend)

This adapts the mandatory review list to concrete checks and tests in this codebase.

#### Arbitrary CPI
- Enforce program ID whitelists for all CPI targets; for SPL Token, ensure `token_program.key` matches expected (Token-2022 if required).
- Derive/verify vaults and authorities locally; do not forward arbitrary remaining accounts to CPI.
- Tests: forged `token_program`, fake vault/authority → expect failure.

#### Improper PDA Validation
- Recompute PDAs with canonical seeds via `find_program_address` and compare to provided keys; ignore user bumps.
- Validate PDAs’ `owner == program_id` and seed specs for market/reserve/referrer PDAs.
- Tests: correct-looking account with wrong bump/seed → reject.

#### Ownership Check
- Enforce `owner == expected_program_id` for program accounts, token accounts, mints, vaults, sysvars.
- Tests: substitute lookalike owned by System or foreign program → fail.

#### Signer Check
- Require `is_signer` on authority accounts where user consent is needed; for CPIs requiring authority, use PDA signer seeds or explicit signer.
- Tests: missing signer should fail.

#### Sysvar Account Check
- Constrain sysvar accounts by address; retrieve via Anchor/SysvarId where possible.
- Tests: fake Sysvar account injected → fail.

#### Improper Instruction Introspection
- When introspecting, search backwards for most recent ix by program ID; validate program ID and expected accounts layout.
- Tests: extra pre/inner instructions to confuse absolute index reads → safe behavior.

Integration Targets
- Token transfers: `utils/token_transfer.rs`, SPL Token-2022 interactions.
- PDA validation: `utils/seeds.rs`, `lending_market/lending_checks.rs`, handler account structs.
- Sysvar usage: instructions sysvar in borrow/repay/flash flows.
- Property tests extend fuzz harness to assert failures on adversarial inputs.


