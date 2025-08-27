### 10 — Core Invariants (Initial Set + usage plan)

We adopt and operationalize the invariants from Appendix A. Each invariant will be bound to specific handlers and validated in tests.

- I1: Reserve vault-accounting conservation
  - Assertion: `vault_balance - available_liquidity` constant pre/post per transfer; expected deltas match action.
  - Coverage: All token-moving handlers; post-transfer reconciliation must run.

- I2: Interest monotonicity
  - Assertion: `cumulative_borrow_rate` non-decreasing; `borrowed_amount_sf` non-decreasing without negative rates.
  - Coverage: Refresh, borrow, repay, liquidation, flash.

- I3: Fee bounds
  - Assertion: protocol/host/referral fees ≥ 0; `absolute_referral_rate_sf <= protocol_take_rate_sf`.
  - Coverage: Borrow interest accrual and fee distribution.

- I4: LTV safety
  - Assertion: For non-liquidation instructions, `loan_to_value() < unhealthy_loan_to_value()` post-exec.
  - Coverage: Borrow, deposit/withdraw, repay, combined flows.

- I5: Price freshness
  - Assertion: If price age > max, sensitive ops fail or conservative flags block them.
  - Coverage: Borrow, withdraw, liquidation.

- I6: Cap enforcement
  - Assertion: Per-interval totals never exceed caps; signed arithmetic safe.
  - Coverage: Deposit/withdraw/borrow on capped reserves.

- I7: Exchange-rate integrity
  - Assertion: Value preserved modulo fees; rounding error bounded ≤ 1 wei per tx.
  - Coverage: Collateral mint/burn paths.

- I8: Authority correctness
  - Assertion: All token movements via correct PDA/signers; reject user-supplied vaults.
  - Coverage: All token-moving handlers.

- I9: Config sanity
  - Assertion: Borrow curve monotonic; fees within [0, 100%]; statuses gate instructions.
  - Coverage: Admin/config updates and status transitions.

Test Strategy
- Encode each invariant as assertions in property tests; collect failing seeds and minimize.
- Map invariants to handlers using the checklist in 05_instruction_checklist.md for coverage tracking.


