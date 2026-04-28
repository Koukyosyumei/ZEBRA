/-
Zebra — Canonicalizer faithfulness for AddSub.

Models the AddSub canonicalizer in `examples/{sp1,ziren,pico,sphinx}/examples/addsub.rs`
(specifically `cr_add` / `cr_sub`) and states the bidirectional theorem that
relates "set of real (b, c, a) tuples extracted from a trace" to the canonical
string output of the full pipeline (`renderSet ∘ map renderTuple ∘ realTuples`).

The canonicalizer currently supports only single-row tables: `cr_add` projects
operand limbs from row 0 unconditionally (`examples/sp1/examples/addsub.rs:28`,
"currently only support one-row table"). This file reflects that invariant.

Three sections, mirroring the discussion:
  (i)   `realTuples` — abstract projection from a trace to the multiset of
        real (b, c, a) tuples; modeled here as a `List Tuple` of length ≤ 1.
  (ii)  `faithful` — the bidirectional theorem.
  (iii) Easy direction proved by congruence; hard direction `sorry`'d, with
        the required injectivity lemmas enumerated as a punch list.

Self-contained: no Mathlib import. Compile with `lean AddSub.lean`.
-/

namespace Zebra.AddSub

/-! ## (i) Trace, Tuple, and `realTuples` -/

/-- An abstract interval over `Int` (mirrors `AbstractInterval` in `src/interval.rs`). -/
structure Interval where
  lo : Int
  hi : Int
deriving DecidableEq, Repr

/-- Display: singleton renders as a bare numeral, otherwise as `[lo, hi]`.
    Mirrors `impl fmt::Display for AbstractInterval` in `src/interval.rs:66`. -/
def Interval.repr (i : Interval) : String :=
  if i.lo = i.hi then toString i.lo
  else "[" ++ toString i.lo ++ ", " ++ toString i.hi ++ "]"

/-- A trace row is a list of intervals indexed by column number. -/
abbrev Row := List Interval

/-- A trace is a list of rows. -/
abbrev Trace := List Row

/-- Project a row at the given column indices.
    Mirrors the index lookup inside `trace_fmt_with_idxs` (`src/trace.rs:123`). -/
def Row.project (r : Row) (idxs : List Nat) : List Interval :=
  idxs.filterMap (fun j => r[j]?)

/-- The AddSub canonical tuple: byte-limb groups for the two inputs and the result. -/
structure Tuple where
  b : List Interval
  c : List Interval
  a : List Interval
deriving DecidableEq, Repr

/-- Configuration: column indices for the three operand groups, plus the
    `is_real` predicate over a row. The sp1 ADD configuration is
    `idxB = [8,9,10,11], idxC = [12,13,14,15], idxA = [1,2,3,4],
    isReal r = (r[17]? ≠ some zeroInterval)`. -/
structure Config where
  idxB   : List Nat
  idxC   : List Nat
  idxA   : List Nat
  isReal : Row → Bool

/-- The set of real tuples extracted from a trace.

    Faithfully models `cr_add`/`cr_sub` (`examples/sp1/examples/addsub.rs:26-54`):
    the loop iterates over all rows to gate via `isReal`, but the projected
    tuple is always taken from row 0 (single-row invariant). The result is
    therefore either empty or a singleton — a `List Tuple` of length ≤ 1
    suffices, and equality on this list coincides with set equality. -/
def realTuples (cfg : Config) (t : Trace) : List Tuple :=
  match t with
  | []        => []
  | row₀ :: _ =>
      if t.any cfg.isReal then
        [{ b := row₀.project cfg.idxB,
           c := row₀.project cfg.idxC,
           a := row₀.project cfg.idxA }]
      else
        []

/-! ## String rendering (mirrors the `format!`/`Display` chain) -/

/-- Mirrors `trace_fmt_with_idxs` (`src/trace.rs:123`): join interval reprs with `", "`. -/
def renderLimbs (xs : List Interval) : String :=
  String.intercalate ", " (xs.map Interval.repr)

/-- Mirrors the `format!` in `cr_add` (`examples/sp1/examples/addsub.rs:30`). -/
def renderTuple (tup : Tuple) : String :=
  "input0: [" ++ renderLimbs tup.b ++
  "], input1: [" ++ renderLimbs tup.c ++
  "], output: [" ++ renderLimbs tup.a ++ "]"

/-- Mirrors `PrettySet::fmt` (`src/utils.rs:66`): brace-wrap a comma+newline
    separated, sorted list. For the single-tuple AddSub case the sort is
    trivial, so we omit it here; a multi-row generalization would sort. -/
def renderSet (strs : List String) : String :=
  "{\n" ++ String.join (strs.map (fun s => s ++ ",\n")) ++ "}"

/-- The full canonicalizer: trace → string. -/
def canonicalize (cfg : Config) (t : Trace) : String :=
  renderSet ((realTuples cfg t).map renderTuple)

/-! ## (ii) The bidirectional faithfulness theorem -/

/-- **Canonicalizer faithfulness.** Two traces yield equal canonical strings iff
    they project to the same multiset of real `(b, c, a)` tuples.

    Stated on `List Tuple` rather than `Finset Tuple`: justified for AddSub
    because `realTuples` returns at most one element under the single-row
    invariant. Multi-row generalization should upgrade to `Multiset` / `Finset`. -/
theorem faithful (cfg : Config) (t₁ t₂ : Trace) :
    realTuples cfg t₁ = realTuples cfg t₂ ↔
    canonicalize cfg t₁ = canonicalize cfg t₂ := by
  refine ⟨?fwd, ?bwd⟩
  /- (iii.a) Easy direction. `canonicalize` is a pure function of `realTuples`,
     so equal projections yield equal strings by congruence. -/
  case fwd =>
    intro h
    show renderSet ((realTuples cfg t₁).map renderTuple)
       = renderSet ((realTuples cfg t₂).map renderTuple)
    rw [h]
  /- (iii.b) Hard direction. Reduces to format-injectivity at three layers.
     Punch list of lemmas to discharge the `sorry`:

       • `Interval.repr_injective : Interval.repr i = Interval.repr j → i = j`
         The singleton-vs-bracketed branch of `Interval.repr` distinguishes
         `{lo = hi}` from `{lo < hi}` outputs (no bracketed form starts with a
         digit-only prefix and no bare numeral starts with `[`). The internal
         comma+space `", "` inside `[lo, hi]` matches `renderLimbs`'s joiner —
         that is the actual ambiguity to discharge: a singleton interval
         rendering "-5" has no comma, but two singletons rendered as a list
         look the same as one bracketed interval iff `Interval.repr` for a
         non-singleton can collide with `"x, y"`. It cannot, because the
         non-singleton form is wrapped in `[...]`.

       • `renderLimbs_injective : renderLimbs xs = renderLimbs ys → xs = ys`
         Follows from `Interval.repr_injective` and the fact that
         `Interval.repr` never produces a string containing the joiner `", "`
         outside of matched brackets `[...]`.

       • `renderTuple_injective : renderTuple s = renderTuple t → s = t`
         Reduces to `renderLimbs_injective` after stripping the constant
         delimiters `"input0: ["`, `"], input1: ["`, `"], output: ["`, `"]"`.
         Soundness requires that `renderLimbs` never emits any of these
         literal substrings — true because `Interval.repr` only emits
         numerals, brackets `[`, `]`, and `, `.

       • `renderSet_injective_on_canonicalized : when both inputs are sorted
         and deduplicated, `renderSet ss = renderSet ts → ss = ts`.
         For the AddSub single-row case this trivializes: both sides are
         length ≤ 1, so list equality is decided by `String.length`-zero.
  -/
  case bwd =>
    sorry

/-! ## Sanity check — concrete instance for sp1 ADD -/

/-- The sp1 ADD column layout, drawn from `cr_add` in
    `examples/sp1/examples/addsub.rs:32-34`. The `isReal` predicate stubs
    column 17; refine when the field model is in place. -/
def sp1AddConfig : Config where
  idxB   := [8, 9, 10, 11]
  idxC   := [12, 13, 14, 15]
  idxA   := [1, 2, 3, 4]
  isReal := fun _ => true  -- stub; real predicate: col 17 nonzero mod prime

/-- Trivial sanity check: the easy direction holds reflexively. -/
example (t : Trace) : canonicalize sp1AddConfig t = canonicalize sp1AddConfig t :=
  (faithful sp1AddConfig t t).mp rfl

end Zebra.AddSub
