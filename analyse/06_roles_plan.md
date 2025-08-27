### 06 — AI Team Structure and Prompts (Operationalized)

Role scoping refined with concrete outputs and dependencies on 01–05.

#### Role 1: Math Analyst
- Output: List of cross-type arithmetic and rounding semantics; proof sketch of interest monotonicity; bounds for slots/rates; negative-delta audit.
- Inputs: `utils/fraction.rs`, `state/reserve.rs`, `state/obligation.rs`, `utils/borrow_rate_curve.rs`.

#### Role 2: Oracle and Price Integrity Analyst
- Output: Map of price-dependent paths; freshness guarantees; adversarial refresh-minimization sequences; tests for stale thresholds.
- Inputs: `lending_market/lending_operations.rs`, `utils/prices/*`, handlers.

#### Role 3: State Machine and Invariants Engineer
- Output: Minimal invariant set; per-handler pre/postconditions; preservation proofs; harness scaffolding.
- Inputs: Handlers, `lending_checks.rs`, state modules.

#### Role 4: Economic Attacker
- Output: Transaction sequences for draining scenarios; flash-assisted strategies; expected deltas.
- Inputs: All price/interest/cap logic; refresh ordering.

#### Role 5: PDA/Auth and Token-2022 Auditor
- Output: PDA list and seed specs; authority flow verification; coverage of Token-2022 validation; negative tests.
- Inputs: `utils/seeds.rs`, `lending_checks.rs`, token utils, handlers.

#### Role 6: Caps and Limits Reviewer
- Output: Cap mapping; proofs/tests for interval arithmetic correctness; bypass attempts across combined flows.
- Inputs: `state/reserve.rs`, `withdrawal_cap_operations.rs`.

#### Role 7: Admin/Config and Governance Reviewer
- Output: Admin lifecycle validation; config range sanity; status transition tests.
- Inputs: `state/global_config.rs`, `handlers/handler_update_*`, `config_items.rs`.

#### Role 8: Fuzzing Engineer
- Output: Property test harness; seed corpus; CI automation.
- Inputs: All above roles’ assertions and oracles; handler maps from 01 and 05.


