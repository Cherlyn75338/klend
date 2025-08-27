### 13 — Fraction Math Deep Dive (`utils/fraction.rs`)

Scope: Fixed-point primitives `Fraction = U68F60`, `BigFraction(U256)`, conversions, arithmetic helpers, and display. This layer underpins interest, fees, and exchange-rate math across `reserve` and `obligation`.

Key Types and Constants
- Fraction type alias: unsigned fixed-point `U68F60` (integer 68 bits, fraction 60 bits).
- `BigFraction(U256)`: wider-precision wrapper to prevent overflow on intermediate products/divisions.
- `FRACTION_ONE_SCALED`, `EPSILON`: scaling helpers.

Core Helpers
- Power and conversions: `pow_fraction`, `bps_u128_to_fraction`, `pct_u128_to_fraction`.
- FractionExtra:
  - Display/format: `to_percent`, `to_bps`, `to_display` (rounded presentation only).
  - Conversions: `from_percent`, `from_bps`, `to_sf/from_sf`, `to_*` and `try_to_*` for floor/ceil/round.
  - Integer ratio multiply: `mul_int_ratio(u128/u128)` uses native Fraction; `full_mul_int_ratio(U256/U256)` lifts to U256 to avoid overflow.
  - `div_ceil` does scaled integer divide with +den-1 trick on scaled bits.

BigFraction Ops
- `Add/Sub/Mul/Div` re-scale by shifting `FRAC_NBITS` as expected.
- `From<T: Into<Fraction>>` packs to U256; `TryFrom<BigFraction> for Fraction` returns `LendingError::IntegerOverflow` on downcast failure.
- Bridges between `U128` and `U256` for intermediate math.

Rounding Semantics Used Elsewhere
- Fee paths typically use `to_round` or `to_floor` depending on borrower/protocol favor.
- Ceiling variants exist for conservative accounting (e.g., repayments).

Identified Risk Points and Notes
- full_mul_int_ratio():
  - Uses U256 intermediates and then `try_into()` to `u128`; on failure it calls `expect("...doesn't fit in a Fraction.")` which will abort the program. Recommend replacing with checked conversion returning a program error (`LendingError::IntegerOverflow`) to avoid panics.
- div_ceil():
  - Computes with `(denum_sf - 1)`; requires `denum > 0`. Callers should never pass zero; assert/require at call sites if not inherently guaranteed.
- BigFraction Mul/Div:
  - No explicit overflow checks inside U256 arithmetic (relies on U256 width). Downcasts via `TryFrom<BigFraction> for Fraction` are checked.
- Display formatting:
  - Presentation-only rounding; not used for state changes, safe.

Cross-File Interactions (context)
- `reserve::compound_interest` uses `BigFraction` and `Fraction` heavily; see 14 for fee/interest monotonicity and the exact rounding impact.
- `obligation::accrue_interest` relies on `BigFraction` cumulative rate; enforces non-decreasing rate via ordering checks (see 15).

Recommendations
- Replace `expect` in `full_mul_int_ratio` with checked error propagation.
- Consider adding `checked_*` variants where plain `-` or `+` on `Fraction` are used in downstream code to prevent aborts in extreme edge cases.


