### 01 — Architecture Map (high-signal files)

This note synthesizes the structure described in `AUDIT_PLAN.md` and maps subsystems, data models, and instruction flows we will reference in subsequent analyses.

#### Top-Level Layout
- **Handlers (`programs/klend/src/handlers/`)**: Entry points per instruction. They orchestrate account validation, freshness checks, token transfers, and delegate core math to lending/state modules.
- **Lending market core (`programs/klend/src/lending_market/`)**: Core operations and checks, including refresh logic, oracle ingestion, caps, and admin/config flows.
- **State (`programs/klend/src/state/`)**: On-chain account types and business logic for interest, exchange rates, LTV, liquidation, and configs.
- **Utils (`programs/klend/src/utils/`)**: Shared math/types, oracle adapters, SPL-Token interactions, PDA seeds, constraints, and validation helpers.

#### Key State Models (and why they matter)
- **`Reserve` (`state/reserve.rs`)**
  - Holds liquidity vault info, fees, borrow curve, exchange rate, caps and limits, status.
  - Critical math: `compound_interest`, collateral exchange rate, fee accrual, caps interval math.
- **`Obligation` (`state/obligation.rs`)**
  - Per-user debts/collateral across reserves; e-mode groupings; LTV and health metrics.
  - Critical math: `accrue_interest`, LTV calculations, variable vs fixed debt handling.
- **`LendingMarket` (`state/lending_market.rs`)**
  - Global parameters: oracle freshness policy, referral fees, elevation groups.
- **`GlobalConfig` (`state/global_config.rs`)**
  - Admin keys, pending admin lifecycle, fee collector.

#### Oracles and Price Path
- **`utils/prices/*`**: Adapters for Pyth, Switchboard, Scope with status/age/confidence checks in `checks.rs` and helpers in `types.rs` and `utils.rs`.
- **Integration point**: `lending_market/lending_operations.rs::refresh_reserve` pulls prices, enforces max age policy, and updates reserve price-dependent fields/flags.

#### Token and Authority Path
- **`utils/token_transfer.rs`, `utils/spltoken.rs`**: CPI wrappers and helpers for Token-2022 operations.
- **`utils/seeds.rs`**: PDA derivations (market authority, reserve vault authority, referrer PDAs, etc.).
- Authority is enforced in handlers and `lending_checks.rs` via ownership/signer/seed validation. Post-transfer reconciliation is centralized in checks.

#### Instruction Families (handlers)
- **Borrow/Repay**: `handler_borrow_obligation_liquidity.rs`, `handler_repay_obligation_liquidity.rs`, `handler_repay_and_withdraw_redeem.rs`
  - Use cases: Increase/decrease user debt; must enforce fresh prices, health checks, and exact token repayment with fee rounding safeguards.
- **Liquidations**: `handler_liquidate_obligation_and_redeem_reserve_collateral.rs`
  - Use cases: Repay unhealthy debt and redeem collateral with bonus bounds.
- **Deposit/Withdraw**: `handler_deposit_reserve_liquidity.rs`, `handler_redeem_reserve_collateral.rs`, `handler_withdraw_obligation_collateral.rs`, `handler_deposit_obligation_collateral.rs`, plus combined paths.
  - Use cases: Mint/burn collateral against vaults at current exchange rate and within caps.
- **Refresh**: `handler_refresh_reserve.rs`, `handler_refresh_obligation.rs`, `handler_refresh_reserves_batch.rs`, farms refreshers.
  - Use cases: Accrue interest per slot, update prices, set flags that gate sensitive ops.
- **Flash loans**: `handler_flash_borrow_reserve_liquidity.rs`, `handler_flash_repay_reserve_liquidity.rs`
  - Use cases: Borrow and repay within a single tx; must enforce fee and exact repayment with no dust gap.
- **Admin/Config**: `handler_update_*`, `handler_init_*`
  - Use cases: Initialize market/reserve/obligation and update configs under admin authority.
- **Accounting/Glue**: Shared validation in `lending_market/lending_checks.rs` including post-transfer vault vs accounting reconciliation.

#### Core Operation Flow (high level)
1) User-facing handler validates accounts, owners, signers, Token-2022 constraints, and PDAs via `lending_checks.rs` and macros.
2) Freshness: Handlers either require recent refresh or invoke refresh logic; prices are sourced via `utils/prices/*` and applied in `refresh_reserve`.
3) Math: Interest accrual and exchange-rate math live in `state/*` with fixed-point utilities in `utils/fraction.rs` and rate curves in `utils/borrow_rate_curve.rs`.
4) Token CPI: Movements are executed via `token_transfer.rs` using program PDAs; then post-transfer checks verify vault/accounting deltas.
5) Invariants: LTV/health, caps, and fee bounds are enforced pre/post within checks and state methods.

#### Cross-Cutting Hotspots (to guide deeper review)
- Interest monotonicity and rounding: `reserve::compound_interest`, `obligation::accrue_interest`, `utils/fraction.rs`.
- Price freshness/status gating: `lending_operations::refresh_reserve` and `utils/prices/checks.rs`.
- LTV and liquidation thresholds vs e-mode: `state/obligation.rs`, `state/liquidation_operations.rs`, `state/lending_market.rs`.
- Collateral exchange-rate precision and mint decimals: `state/reserve.rs`.
- Caps (withdraw/borrow) and interval arithmetic: `state/reserve.rs`, `lending_market/withdrawal_cap_operations.rs`.
- Flash loan fee and exact repayment checks: flash handlers + `lending_checks.rs`.
- Token-2022 extension validation and post-transfer accounting: `lending_checks.rs`, `utils/token_transfer.rs`.
- PDA/authority validation: `utils/seeds.rs`, `lending_checks.rs`, and all handlers that sign for vaults.

#### Dependencies and Data Flow (mental model)
- Handlers -> `lending_checks.rs` (validation) -> State methods (`reserve`, `obligation`) for math -> Token CPI via PDAs -> Post-transfer checks.
- Prices flow: Oracle adapters -> `refresh_reserve` -> reserve fields/flags -> consumed by borrow/withdraw/liquidation eligibility.

#### Artifacts for Later Sections
- We will reference this map when:
  - Enumerating threat actors and surfaces (02 Threat Model) with modules tied to each class.
  - Prioritizing high-risk areas (03) by files and functions listed above.
  - Planning tests and fuzz harness touchpoints (04 Methodology) along the flow numbered above.


