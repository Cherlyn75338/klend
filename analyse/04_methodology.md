### 04 — Methodology and Workflow

This operational plan ties Sections 0–7 into concrete steps and artifacts, leveraging insights from 01–03.

#### Phase 0: Build and Baseline
- Build with Anchor/Cargo; run existing tests in `tests/klend.ts`.
- Generate a minimal call map: handlers → checks → state methods; list price-dependent paths.
- Extract pre/postconditions per handler from `lending_checks.rs` and account structs.

Artifacts:
- Handler contract matrix (accounts, freshness, authority, price usage).
- Initial invariant mapping (see 10_invariants.md template).

#### Phase 1: Manual Review (Deep Dive)
- Trace state writes to `Reserve`, `Obligation`, `LendingMarket`, `GlobalConfig` for borrow/repay/liquidate/deposit/withdraw/refresh.
- Verify price freshness gating; confirm rounding and overflow boundaries in math utilities.
- Cross-check borrow vs repay vs liquidation for consistency and invariant preservation.

Artifacts:
- Per-handler notes with identified checks and potential gaps.

#### Phase 2: Property-Based and Differential Testing
- Compose randomized instruction sequences with constraints (multi-user/reserve, price paths, time steps).
- Assertions: conservation, monotonicity, safety, oracle freshness, caps, and no net loss without liability.

Artifacts:
- Fuzz harness plan; seed capture and minimization strategy; CI integration.

#### Phase 3: Numeric Analysis
- Range analysis on fixed-point arithmetic; identify max safe slot deltas and rates.
- Prove fee bounds and non-negativity; ensure cross-type conversions do not overflow.

#### Phase 4: Oracle Integrity Tests
- Simulate max-age windows; validate conservative flags block sensitive ops when stale.

#### Phase 5: Abuse Sequences
- Flash-assisted sequences; stale exchange-rate combined ops; liquidation bonus edges.

#### Phase 6: Admin/Governance
- Pending admin lifecycle tests; reserve status transition gating; config value ranges.

#### Phase 7: PDA/Auth and Token-2022
- Validate PDA derivations and token authority flows; attempt malicious extensions and expect rejection.

#### Phase 8: Safety Nets
- Layout stability of `#[zero_copy]` structs; safe error handling replacing reachable unwraps.

Dependencies:
- Uses `01_architecture_map.md` for module references.
- Validates hypotheses listed in `03_high_risk_areas.md`.


