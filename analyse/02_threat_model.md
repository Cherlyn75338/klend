### 02 — Threat Model (Solana + Lending specifics)

This section refines the plan’s threat model with concrete attack surfaces mapped to modules from 01_architecture_map.

#### Adversary Goals → Surfaces
- **Over-borrow / under-repay**
  - Surfaces: Borrow/Repay handlers, `state/obligation.rs::accrue_interest`, `state/reserve.rs::compound_interest`, `lending_checks.rs` post-transfers, flash loan handlers.
- **Manipulate interest/fees**
  - Surfaces: `utils/fraction.rs`, fee splitting in `state/reserve.rs`, referral in `state/lending_market.rs`.
- **Exploit stale/bad prices**
  - Surfaces: `lending_market/lending_operations.rs::refresh_reserve`, `utils/prices/*`.
- **Bypass caps/limits**
  - Surfaces: `state/reserve.rs` caps, `lending_market/withdrawal_cap_operations.rs`.
- **Admin misconfig/authority misuse**
  - Surfaces: `state/global_config.rs`, `handler_update_*`, PDA derivations `utils/seeds.rs`.
- **Reentrancy-like sequencing / flash interactions**
  - Surfaces: combined handlers (repay+withdraw), flash borrow/repay same slot; refresh ordering.
- **Token accounting mismatches**
  - Surfaces: token CPI wrappers and `post_transfer_vault_balance_liquidity_reserve_checks`.

#### Capabilities
- Full control of user accounts, ordering of instructions within a transaction, ability to include flash loans, stale or crafted oracles within allowed policy, and crafted remaining accounts.
- Use of tokens with edge-case decimals or Token-2022 extensions.

#### Trust/Assumptions
- Program code and whitelisted oracle programs (Pyth/Switchboard/Scope), along with PDA-controlled vaults and authorities.

#### Primary Risks and Hypotheses
1) **Interest monotonicity violation** enabling debt decreases via non-monotonic rate updates or rounding.
2) **Stale price acceptance** allowing unsafe borrow/withdraw under outdated valuation.
3) **LTV/E-mode inconsistencies** where borrow-factor basis differs from unhealthy thresholds.
4) **Collateral exchange rate drift** from rounding or decimals extremes.
5) **Cap arithmetic errors** causing underflow/overflow or interval miscounts.
6) **Flash loan dust under-repayment** due to rounding or fee sentinel paths.
7) **Token-2022 extension gaps** permitting unauthorized behavior.
8) **Admin misconfiguration** that opens drain or disables safety flags.
9) **PDA/account spoofing** leading to fund redirection.

#### Defensive Controls (expected)
- Freshness gating macros/checks before sensitive ops; price status flags with conservative fallbacks.
- Centralized post-transfer accounting checks; strict Token-2022 validation.
- PDA recomputation and strict owner/signer checks; instruction sysvar constrained where used.
- Caps enforced with interval logic; exchange-rate computed via fixed-point with bounded rounding.

#### Evidence/Artifacts Needed (to collect later)
- List of all borrow/repay and combined-path checks that rely on prices and freshness.
- Exact rounding semantics in `fraction.rs` and conversion boundaries.
- Cap update logic tests around boundary timestamps.
- Token-2022 extension validation coverage map.

See 01 for module mapping; subsequent sections will validate these hypotheses with code reads and tests.


