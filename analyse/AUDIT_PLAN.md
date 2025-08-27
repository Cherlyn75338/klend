## Klend Protocol Audit Strategy (Coach Plan)

### Objective
- Identify deep, economically meaningful vulnerabilities in `programs/klend/src` that could enable fund theft, insolvency, broken invariants, or systemic loss via math/logic errors, price manipulation, authorization bugs, or accounting mismatches.
- Produce proofs of exploitability (POCs/fuzz seeds) and formalized invariants.

### Scope
- Primary: Solana on-chain program under `programs/klend/src` (handlers, state, lending_market, utils). Focus on:
  - Interest accrual, fee accounting, and debt/collateral math.
  - Price oracle ingestion and staleness handling.
  - LTV/liquidation logic, collateral exchange rates, and withdrawal/borrow caps.
  - Token accounting vs. vault balances; flash loan path; socialize-loss flows.
  - Admin/config updates and authority controls.
  - PDA derivations and account validation; Token-2022 extensions.
- Secondary: Tests and migrations for usage assumptions and invariants.

### Architecture Map (high-signal files)
- Handlers (`programs/klend/src/handlers/`): entrypoints
  - Borrow/Repay: `handler_borrow_obligation_liquidity.rs`, `handler_repay_obligation_liquidity.rs`, `handler_repay_and_withdraw_redeem.rs`
  - Liquidations: `handler_liquidate_obligation_and_redeem_reserve_collateral.rs`
  - Deposits/Withdrawals: `handler_deposit_reserve_liquidity.rs`, `handler_redeem_reserve_collateral.rs`, `handler_withdraw_obligation_collateral.rs`, `handler_deposit_obligation_collateral.rs`, combo paths
  - Refresh: `handler_refresh_reserve.rs`, `handler_refresh_obligation.rs`, farms refreshers
  - Flash loans: `handler_flash_borrow_reserve_liquidity.rs`, `handler_flash_repay_reserve_liquidity.rs`
  - Admin/Config: `handler_update_*`, `handler_init_*`
  - Accounting/glue: post-transfer checks in checks layer
- Lending market core (`programs/klend/src/lending_market/`)
  - `lending_operations.rs`: `refresh_reserve` (interest accrual + price updates), price-age checks
  - `lending_checks.rs`: account validation and post-conditions; Token-2022 extension checks; vault/accounting sanity
  - `ix_utils.rs`, `config_items.rs` (config mutation), `withdrawal_cap_operations.rs`, `farms_ixs.rs`, `flash_ixs.rs`
- State (`programs/klend/src/state/`)
  - `reserve.rs`: reserve config, fees, interest `compound_interest`, exchange rates, limits/caps
  - `obligation.rs`: user debt/collateral state, `accrue_interest`, LTV calculations, unhealthy thresholds
  - `lending_market.rs`: global market config (price refresh policy, referral fees)
  - `global_config.rs`: global admin/pending admin/fee collector
  - `liquidation_operations.rs`: liquidation eligibility and bonus computation
  - `types.rs`, `token_info.rs`, `referral.rs`
- Utils (`programs/klend/src/utils/`)
  - `fraction.rs`: fixed-point math (`U68F60`), BigFraction/U256 bridges, rounding helpers
  - `borrow_rate_curve.rs`: piecewise-linear borrow curve, validation
  - `prices/`: `pyth.rs`, `switchboard.rs`, `scope.rs`, `types.rs`, `checks.rs` (price ingestion, status)
  - `token_transfer.rs`, `spltoken.rs`, `constraints.rs`, `validation.rs`

---

## Threat Model (Solana + Lending specifics)
- Adversary goals: extract value by borrowing more than allowed; repay less than owed; manipulate interest/fees; exploit stale/bad prices; bypass caps or accounting; steal via admin misconfig; reentrancy-like cross-instruction sequencing; flash-loan assisted draining; oracle liveness gaps.
- Capabilities:
  - Control of user accounts; arbitrary instruction ordering within a transaction; flash liquidity; ability to pass crafted remaining-accounts; race with refreshes; choosing oracles within allowed policy.
  - Malicious mints or tokens with edge decimals or extensions; abusing Token-2022 features if not checked.
- Trust:
  - Program code; SPL/Anchor semantics; whitelisted oracle programs (Pyth/Switchboard/Scope) per policy; admin/controller keys.

---

## High-Risk Areas and Hypotheses
1) Interest and Fee Math
- Files: `state/reserve.rs::compound_interest`, `state/obligation.rs::accrue_interest`, `utils/fraction.rs`, `utils/borrow_rate_curve.rs`.
- Risks:
  - Non-monotonic cumulative rate updates creating negative interest deltas (there is a guard in `obligation::accrue_interest`, verify reserve path too).
  - Rounding direction enabling “dust farming” to skim value over loops; cross-component rounding inconsistencies between Reserve vs Obligation.
  - Overflow/underflow across U128/U256 boundaries in BigFraction conversions or `full_mul_int_ratio`.
  - Protocol/host/referral fee split miscalculation permitting protocol fee bleed or referrer overpayment; sanity vs `u64::MAX` sentinels.
- Targets:
  - Prove that debt never decreases when rates increase; fees non-negative; totals conserved modulo expected sinks/sources; `absolute_referral_rate_sf` bounded in [0, protocol_take_rate].

2) Price Oracles and Staleness
- Files: `lending_market/lending_operations.rs::refresh_reserve`, `utils/prices/*`.
- Risks:
  - Stale but “valid” saved price usage path; `is_saved_price_age_valid` thresholds; window where price not refreshed but used.
  - Status flags and missing fallback; ensure unset price sets conservative flags preventing unsafe borrow/withdraw.
  - Cross-reserve price skew enabling bad debt via cross-asset liquidation.
- Targets:
  - Confirm that any operation using price requires freshness or safe fallback constraints; test max-age and trigger thresholds; assert conservative behavior on missing price.

3) LTV / Liquidation / E-Mode / Borrow Factors
- Files: `state/obligation.rs` (LTV functions), `state/liquidation_operations.rs`, `state/reserve.rs` (borrow_factor, disable usage outside e-mode), `state/lending_market.rs` (emode configs).
- Risks:
  - Inconsistent LTV basis: borrow-factor-adjusted vs unhealthy values; ordering of updates vs checks.
  - E-mode limits bypass: mixing elevation group assets to escape per-asset or group limits.
  - Liquidation bonus computation leading to over-redemption; min/max bonus bounds not enforced uniformly.
- Targets:
  - Prove `loan_to_value() <= unhealthy_loan_to_value()` under all state transitions; verify liquidation bonus within configured [min, max, emode-max].

4) Collateral and Exchange Rates
- Files: `state/reserve.rs` (collateral.exchange_rate, mint decimals checks).
- Risks:
  - Rounding during mint/burn collateral leading to value drift; decimals edge cases; `mint decimals < 20` assumption.
  - Withdrawals at stale exchange rate; sequence attacks across refresh boundaries.

5) Withdrawal/Borrow Caps and Limits
- Files: `state/reserve.rs` (`WithdrawalCaps`, `deposit_limit`, `borrow_limit`), `lending_market/withdrawal_cap_operations.rs`.
- Risks:
  - Interval arithmetic on caps (`current_total`, timestamps) underflow when converting signed/unsigned; incorrect reset boundary; bypass via multi-instruction ordering.

6) Flash Loans
- Files: `handlers/handler_flash_*`, `lending_checks.rs`.
- Risks:
  - Zero-fee path if fee sentinel mis-set; repayment check precision/rounding enabling dust shortfall; interactions with interest accrual in same slot.

7) Token Accounting vs Vault Balances
- Files: `lending_checks.rs::post_transfer_vault_balance_liquidity_reserve_checks`.
- Risks:
  - Missing calls on some paths; assumptions invalid for Token-2022 extensions; mint/ATA mismatches.
  - Signed subtraction path `SubstractiveSigned` correctness and conversions.

8) Admin and Config
- Files: `state/global_config.rs`, `handlers/handler_update_*`, `lending_market/config_items.rs`.
- Risks:
  - Pending admin application semantics; partial updates leaving system in unsafe states; fee collector redirection; reserve status updates enabling stealth drains.

9) PDA, Seeds, and Account Validation
- Files: `utils/seeds.rs`, `lending_checks.rs::validate_referrer_token_state`, handlers.
- Risks:
  - Incorrect seeds or missing signer checks for authority PDAs; ability to pass foreign reserve/obligation with matching shape; remaining-accounts spoofing.

10) Farms, Rewards, Socialize Loss
- Files: handlers for farms refresh, `handler_socialize_loss.rs`.
- Risks:
  - Adjustments that touch debt/collateral not gated by invariants; socialize-loss enabling loss shifting across users arbitrarily.

---

## Methodology and Workflow

### Phase 0: Build, Baseline, and Spec Extraction
- Build with Anchor/Cargo, run tests, and generate a minimal call graph of handlers → state mutations.
- Auto-extract function contracts:
  - Preconditions: account constraints, refresh requirements, signer/authority checks.
  - Postconditions: state diffs, expected balance changes, invariant preservation.
- Document invariants (see Appendix A) and bind them to each instruction.

### Phase 1: Manual Code Review (Deep Dive)
- Walk each high-value handler and its checks, tracing:
  - All reads/writes to `Reserve`, `Obligation`, `LendingMarket`, `GlobalConfig`.
  - Price usage points and freshness checks.
  - Any math overflow boundary and rounding direction, especially converting between Fraction/U256/U64.
- Cross-compare borrow vs repay vs liquidation paths for consistency.
- Confirm all sensitive paths invoke `check_refresh_ixs!` or equivalent freshness checks where required.

### Phase 2: Property-Based and Differential Testing
- Build a fuzz harness that composes instruction sequences with realistic constraints:
  - Randomized users, reserves, mints (with varying decimals including extremes), price paths (bounded volatility), and time/slot steps.
  - Operations sequences: [refresh]* → deposit → borrow → refresh → repay → withdraw; include permutations and concurrent reserves.
  - Inject adversarial elements: stale price windows, flash-loan-included transactions, partial repays, near-cap boundaries, e-mode toggles, and signed-cap arithmetic around zero.
- Properties to assert (sample):
  - Conservation: reserve.vault_diff == available_diff across transfers; program balance changes match token moves.
  - Monotonicity: cumulative_borrow_rate non-decreasing; interest and fees non-negative.
  - Safety: `user_ltv <= unhealthy_ltv` after all non-liquidation instructions; cannot borrow if `> max`.
  - Oracle safety: if price is stale/invalid, operations that depend on price must fail or use conservative bounds.
  - Caps: withdrawals/borrows never exceed configured caps over intervals.
  - No single-tx sequence yields net program loss without corresponding user liability increment.

### Phase 3: Symbolic/Range Numeric Analysis
- Use range analysis on `Fraction` arithmetic:
  - Inputs: rates in [0, 100%], utilization in [0, 100%], slots up to realistic bounds.
  - Show no overflow in BigFraction operations; rounding errors bounded and not exploitable by loops.
  - Prove `absolute_referral_rate_sf <= protocol_take_rate_sf` and non-negativity of fee components.

### Phase 4: Oracle and Price Integrity Tests
- Simulate price drift within max-age window; ensure `refresh_reserve` sets conservative flags when price is too old.
- Verify all borrow/withdraw/liquidation paths read price only after ensured freshness or handle None/empty status safely.

### Phase 5: Abuse Scenarios and Sequences
- Flash-loan assisted borrow/repay in same slot with strategic refresh ordering to reduce fees or debt.
- Collateral withdraw with stale exchange rate; re-enter via combined instructions (`repay_and_withdraw_redeem`) to bypass LTV.
- Multi-reserve liquidation exploiting bonus computation boundaries across emode limits.
- Referral fee inflation by crafting borrowed variable portion while minimizing protocol fee via rounding.

### Phase 6: Admin/Governance Hardening
- Attempt to misconfigure fee collector and pending admin transitions; ensure only admin can apply and no front-running window to steal.
- Reserve status transitions (`Active/Obsolete/Hidden`) do not allow borrow or value extraction during transition boundaries.

### Phase 7: PDA/Auth and Token-2022
- Validate all PDAs and seeds; ensure all token movements require program authority or correct PDA signer seeds; reject foreign vaults.
- Token-2022: verify `validate_liquidity_token_extensions` coverage on all relevant paths; attempt to use forbidden extensions.

### Phase 8: Code Quality and Safety Nets
- Ensure `#[zero_copy]` structs remain layout-stable; padding fields not misused.
- Check all conversions (`try_into`, `unwrap`) near external inputs; replace `unwrap` with safe errors where reachable.

---

## Instruction-by-Instruction Checklist (Condensed)
- Borrow (`handler_borrow_obligation_liquidity.rs`)
  - Enforce fresh reserve and obligation (macro usage check).
  - Validate destination not the supply vault; version/status checks; Token-2022 extension check.
  - Ensure borrow math uses current price; check LTV and per-reserve/group borrow limits; verify post-transfer accounting.
- Repay (`handler_repay_obligation_liquidity.rs`, combo `repay_and_withdraw`)
  - Check repay source not reserve vault; version checks; refresh conditions.
  - Ensure partial repay updates interest correctly; when combined with withdraw, confirm LTV check runs after repay effects.
- Deposit/Withdraw liquidity and collateral
  - Prevent using vaults as user sources/destinations; version/status checks; exchange rate correctness; cap enforcement and interval updates.
- Liquidation and Redeem
  - Validate eligibility (`check_liquidate_obligation`); enforce liquidation bonus bounds; ensure repay <= max; accounting after redeem.
- Flash Borrow/Repay
  - Fee enabled and charged; ensure no rounding-based under-repay; slot-level interactions with interest accrual.
- Refresh Reserve/Obligation
  - Accrue interest once per slot; price path updates and max age gating; clear reserved fields as expected.
- Admin/Config Updates
  - Only admin; pending admin apply; validate `UpdateReserveConfigValue::Full` and individual item bounds; disallow dangerous curves/fees.

---

## AI Team Structure and Prompts

### Role 1: Math Analyst (Fixed-Point and Fees)
- Mission: Prove safety/monotonicity of interest, debt, and fee math; find rounding exploits.
- Focus Files: `utils/fraction.rs`, `state/reserve.rs::compound_interest`, `state/obligation.rs::accrue_interest`, `utils/borrow_rate_curve.rs`.
- Prompts:
```text
You are the Math Analyst. Read utils/fraction.rs, state/reserve.rs (compound_interest), state/obligation.rs (accrue_interest), and utils/borrow_rate_curve.rs. Produce:
1) A list of all arithmetic expressions that cross U128/U256 or BigFraction boundaries; annotate rounding direction and potential overflow points.
2) A proof sketch that cumulative_borrow_rate is non-decreasing and that new_debt_f >= previous_debt_f when rates >= 0.
3) Bounds for max slots_elapsed and rates that keep all intermediates within safe ranges. Provide counterexamples if any.
4) Identify any place where subtraction between fractional values can go negative (e.g., net_new_variable_debt_f) and explain why it cannot under valid inputs, or produce a failing sequence.
5) Recommend exact rounding (ceil/floor) required to avoid borrower-favorable skimming loops.
```

### Role 2: Oracle and Price Integrity Analyst
- Mission: Ensure no price staleness path enables unsafe operations; detect cross-reserve price skew exploits.
- Focus Files: `lending_market/lending_operations.rs`, `utils/prices/*`.
- Prompts:
```text
You are the Oracle Analyst. Map all instruction paths that depend on prices. For each, answer:
1) Where does the code guarantee a fresh price (slot/time)? What happens if price is missing or stale?
2) Show any path where is_saved_price_age_valid() returning true allows an outdated price to be used for borrow/withdraw.
3) Construct adversarial sequences that minimize refresh calls before sensitive ops.
4) Validate that price status flags (e.g., confidence) influence operation gating.
5) Provide tests that simulate stale prices within thresholds to validate conservative behavior.
```

### Role 3: State Machine and Invariants Engineer
- Mission: Extract invariants; prove they hold across all instruction sequences.
- Focus Files: handlers for borrow/repay/deposit/withdraw/liquidate/refresh; state structs.
- Prompts:
```text
You are the Invariants Engineer. Deliver:
1) A minimal set of invariants covering solvency, LTV bounds, accounting conservation, and caps.
2) For each handler, specify preconditions and postconditions, and prove (manually + property tests) invariant preservation.
3) Identify any handler missing post-transfer vault/accounting checks.
4) Provide a harness to simulate multi-reserve, multi-user sequences with randomized ordering.
```

### Role 4: Abuse Sequences and Economic Attacker
- Mission: Craft realistic draining sequences exploiting ordering, rounding, and price timing.
- Prompts:
```text
You are the Economic Attacker. Produce:
1) A list of transaction-level sequences (with refresh placement) that could reduce effective repay or inflate borrow capacity.
2) Flash-loan sequences that take advantage of same-slot interest behavior.
3) Collateral withdraw at stale exchange rate combined with repay to bypass LTV.
4) Cross-reserve liquidation with bonus edge-case to over-redeem.
Provide concrete sequences and expected deltas.
```

### Role 5: PDA/Auth and Token-2022 Auditor
- Mission: Validate PDAs, signers, seeds; ensure Token-2022 extension validation coverage.
- Prompts:
```text
You are the PDA/Auth Auditor. Deliver:
1) A list of all PDAs, their seeds, and authority flows for token transfers.
2) Confirm every token movement uses the correct authority (program or PDA) and that user-supplied accounts cannot redirect funds.
3) List all instruction handlers that use Token-2022 `validate_liquidity_token_extensions` and identify any gaps.
4) Propose tests that try malicious extensions (e.g., transfer hooks) to ensure rejection.
```

### Role 6: Caps and Limits Reviewer
- Mission: Verify deposit/borrow/withdrawal caps and interval arithmetic.
- Prompts:
```text
You are the Caps Reviewer. Map all cap checks and interval updates. Prove that:
1) current_total never underflows/overflows even with signed arithmetic.
2) Interval resets happen exactly at boundaries; no double-counting.
3) Combined instructions cannot bypass caps by interleaving actions.
Provide property tests around boundary timestamps.
```

### Role 7: Admin/Config and Governance Reviewer
- Mission: Ensure only-authorized updates; no unsafe transient states.
- Prompts:
```text
You are the Admin Reviewer. Validate:
1) Pending admin lifecycle and apply semantics; no ability for non-admin to front-run.
2) Reserve config updates (Full and granular) enforce value ranges (e.g., borrow_curve monotonicity, fees bounds).
3) Status transitions prevent borrow on Obsolete/Hidden as intended.
Propose unit tests for each mode.
```

### Role 8: Fuzzing Engineer
- Mission: Build and run the fuzz/property test harness; generate seeds and minimize counterexamples.
- Prompts:
```text
You are the Fuzzer. Implement a proptest/anchor-client based harness that composes random instruction sequences across multiple users/reserves with constraints. Emit invariants as assertions. Produce failing seeds and minimize them. Automate across nightly CI.
```

---

## Deliverables
- Invariants document and mapping to instructions.
- Fuzz harness and targeted unit/integration tests.
- Vulnerability findings with:
  - Impact analysis and exploitability.
  - POC transactions or fuzz seeds and deltas.
  - Remediation guidance and suggested code edits.
- Risk register for admin/config items with safe ranges.

## Execution Cadence
- Day 1–2: Phase 0/1 baselining and invariant draft.
- Day 3–5: Phase 2 fuzz/property tests and Phase 3 numeric analysis.
- Day 6–7: Oracle/economic attack scenarios and admin hardening.
- Day 8: Consolidate findings, POCs, and remediations.

---

## Reporting Format
- Title, Severity, Affected Files/Functions
- Description and Root Cause
- Exploit Scenario (steps/sequence)
- Impact and Conditions
- POC (transaction or seed)
- Recommended Fix (code-level guidance)
- Tests to Prevent Regression

---

## Appendix A: Core Invariants (Initial Set)
- I1: Reserve vault-accounting conservation: for any transfer, `vault_balance - available_liquidity` constant pre/post; and expected deltas match action.
- I2: Interest monotonicity: `cumulative_borrow_rate` non-decreasing; `borrowed_amount_sf` non-decreasing absent negative rates.
- I3: Fee bounds: protocol/host/referral fees ≥ 0; `absolute_referral_rate_sf <= protocol_take_rate_sf`.
- I4: LTV safety: post non-liquidation, `loan_to_value() < unhealthy_loan_to_value()`; liquidations only when `>=` threshold.
- I5: Price freshness: if price age > max, borrowing/withdrawing requiring price must fail or use conservative status that blocks risk.
- I6: Cap enforcement: per-interval totals never exceed caps; signed arithmetic safe.
- I7: Exchange-rate integrity: collateral/liq exchange math preserves total value modulo fees and rounding bounded by ≤ 1 wei per tx.
- I8: Authority: all token movements via correct PDA/signers; user-supplied vaults rejected.
- I9: Config sanity: borrow curve monotonic; fees within [0, 100%]; statuses gate instructions as intended.

## Appendix B: Files to Prioritize
- `programs/klend/src/state/reserve.rs`
- `programs/klend/src/state/obligation.rs`
- `programs/klend/src/utils/fraction.rs`
- `programs/klend/src/utils/borrow_rate_curve.rs`
- `programs/klend/src/lending_market/lending_operations.rs`
- `programs/klend/src/lending_market/lending_checks.rs`
- `programs/klend/src/utils/prices/*`
- `programs/klend/src/handlers/*` (borrow/repay/liquidate/flash/deposit/withdraw/refresh)

## Standard Solana Bug-Class Checklist (Mandatory Review)

This section adds explicit checks, tests, and references for common Solana bug classes to be applied across all handlers and state mutations.

### Arbitrary CPI
- Risks: Program performs CPI to an attacker-controlled program or passes attacker-controlled accounts to CPI, enabling arbitrary effects.
- Required checks:
  - Whitelist CPI target program IDs; compare to hardcoded expected IDs before any CPI.
  - For SPL Token CPI, enforce `token_program.key == spl_token_2022::ID` (or expected)
    and reject user-supplied alternatives.
  - Do not forward user-supplied remaining accounts blindly to CPIs; derive/verify all vaults and authorities (PDAs) locally.
- Tests:
  - Attempt CPI with a forged `token_program` or other program; expect failure.
  - Attempt to pass a fake vault or authority account to CPI; expect failure.
- Reference: `Arbitrary CPI` example [link](https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/arbitrary_cpi).

### Improper PDA Validation
- Risks: PDAs spoofed via user-provided bump or partial seed control.
- Required checks:
  - Recompute PDA with `Pubkey::find_program_address` from canonical seeds; compare equality to provided account key.
  - Never trust a user-provided bump; ignore and recompute internally.
  - For referrer token state and market/reserve authorities, validate seeds match spec and `account.owner == program_id`.
- Tests:
  - Provide the correct-looking account with wrong bump/seed; expect rejection.
- Reference: `Improper PDA Validation` [link](https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/improper_pda_validation).

### Ownership Check
- Risks: Missing `owner` validation allows attacker to supply accounts owned by another program or EOAs.
- Required checks:
  - Enforce `account.owner == expected_program_id` for: reserves/obligations (our program), token accounts (SPL Token-2022), mints, vaults, sysvars.
  - For PDAs, ensure `owner == program_id`.
- Tests:
  - Substitute a lookalike account owned by `SystemProgram` or foreign program; expect failure.
- Reference: `Ownership Check` [link](https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/ownership_check).

### Signer Check
- Risks: Not requiring `is_signer` for authority accounts allows unauthorized state changes or token transfers.
- Required checks:
  - For any instruction that requires user consent (e.g., creating obligations, moving user tokens), require the appropriate signer(s).
  - For CPIs that require an authority, ensure the authority is either the program PDA (with seeds) or a signer, never a plain account.
- Tests:
  - Attempt the instruction without required signers; expect failure.
- Reference: `Signer Check` [link](https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/signer_check).

### Sysvar Account Check
- Risks: User supplies a spoofed sysvar account, breaking time/slot/IX introspection assumptions.
- Required checks:
  - Constrain sysvar accounts by address (`#[account(address = sysvar::instructions::ID)]` or explicit key checks).
  - Retrieve sysvars via Anchor or `SysvarId` where possible; reject user-supplied alternatives.
- Tests:
  - Provide a fake `Sysvar` account; expect failure.
- Reference: `Sysvar Account Check` [link](https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/sysvar_account_check).

### Improper Instruction Introspection
- Risks: Accessing `Instructions` sysvar by absolute index (TOCTOU, inner-IX confusion), enabling spoofing of prior instructions.
- Required checks:
  - When relying on introspection, search backwards from current index to find the most recent instruction by program ID, rather than using absolute index.
  - Validate the instruction program ID and expected accounts layout before trusting its data.
- Tests:
  - Craft transactions with extra pre-instructions and inner instructions to confuse absolute index reads; expect safe behavior.
- Reference: `Improper Instruction Introspection` [link](https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/improper_instruction_introspection).

### Integration Into This Codebase
- Apply the above checks to all handlers, with emphasis on:
  - CPI paths in token transfers (`utils/token_transfer.rs`, SPL Token-2022 interactions).
  - PDA validation for `lending_market_authority`, reserve vault authorities, referrer token state PDAs.
  - Ownership/signer constraints in `lending_market/lending_checks.rs` and handler account structs.
  - Sysvar usage: `SysInstructions` in borrow/repay/flash flows; ensure constrained addresses.
- Extend property/fuzz tests to assert these checks under adversarial inputs.

### Citations
- Arbitrary CPI: `https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/arbitrary_cpi`
- Improper Instruction Introspection: `https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/improper_instruction_introspection`
- Improper PDA Validation: `https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/improper_pda_validation`
- Ownership Check: `https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/ownership_check`
- Signer Check: `https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/signer_check`
- Sysvar Account Check: `https://github.com/crytic/building-secure-contracts/tree/master/not-so-smart-contracts/solana/sysvar_account_check`
