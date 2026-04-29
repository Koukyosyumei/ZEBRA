/-
Zebra — quotient theory for canonical trace-table spaces.
-/
import Mathlib.Data.Quot
import Zebra.Canonicalizer.Generic

namespace Zebra.Generic

/-- Semantic equivalence induced by the canonicalizer: two raw trace tables are
    equivalent exactly when canonicalization returns the same representation set. -/
def SemanticEquiv {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (t₁ t₂ : Trace) : Prop :=
  canonicalize cfg t₁ = canonicalize cfg t₂

theorem semanticEquiv_refl {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (t : Trace) :
    SemanticEquiv cfg t t := rfl

theorem semanticEquiv_symm {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) {t₁ t₂ : Trace}
    (h : SemanticEquiv cfg t₁ t₂) :
    SemanticEquiv cfg t₂ t₁ := h.symm

theorem semanticEquiv_trans {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) {t₁ t₂ t₃ : Trace}
    (h₁₂ : SemanticEquiv cfg t₁ t₂)
    (h₂₃ : SemanticEquiv cfg t₂ t₃) :
    SemanticEquiv cfg t₁ t₃ := Eq.trans h₁₂ h₂₃

/-- Setoid of raw trace tables modulo semantic equivalence. -/
def semanticSetoid {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) : Setoid Trace where
  r := SemanticEquiv cfg
  iseqv := ⟨semanticEquiv_refl cfg, semanticEquiv_symm cfg, semanticEquiv_trans cfg⟩

/-- Canonical trace-table space `D / ~R`. -/
abbrev CanonicalTraceSpace {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) := Quotient (semanticSetoid cfg)

/-- Image of the canonicalizer `Im(R)`. -/
abbrev CanonicalizerImage {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) := { reprs : Finset Repr // ∃ table, canonicalize cfg table = reprs }

/-- The natural map from canonical trace classes to canonicalizer image. -/
noncomputable def quotientToImage {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) :
    CanonicalTraceSpace cfg → CanonicalizerImage cfg :=
  Quotient.lift
    (fun table => ⟨canonicalize cfg table, ⟨table, rfl⟩⟩)
    (by
      intro t₁ t₂ h
      apply Subtype.ext
      exact h)

/-- The natural map from canonicalizer image back to canonical trace classes,
    choosing any raw table that realizes the image element. -/
noncomputable def imageToQuotient {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) :
    CanonicalizerImage cfg → CanonicalTraceSpace cfg :=
  fun image => Quotient.mk (semanticSetoid cfg) (Classical.choose image.property)

theorem quotientToImage_imageToQuotient {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (image : CanonicalizerImage cfg) :
    quotientToImage cfg (imageToQuotient cfg image) = image := by
  apply Subtype.ext
  exact Classical.choose_spec image.property

theorem imageToQuotient_quotientToImage {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) (q : CanonicalTraceSpace cfg) :
    imageToQuotient cfg (quotientToImage cfg q) = q := by
  refine Quotient.inductionOn q ?_
  intro table
  apply Quotient.sound
  exact Classical.choose_spec (quotientToImage cfg (Quotient.mk (semanticSetoid cfg) table)).property

/-- Formal version of the paper statement: the quotient of raw trace tables by
    canonicalizer-induced semantic equivalence is naturally bijective with the
    image of the canonicalizer. -/
theorem canonicalTraceSpace_equiv_image {Repr : Type} [DecidableEq Repr]
    (cfg : Config Repr) :
    Function.Bijective (quotientToImage cfg) := by
  constructor
  · intro q₁ q₂ h
    rw [← imageToQuotient_quotientToImage cfg q₁,
        ← imageToQuotient_quotientToImage cfg q₂,
        h]
  · intro image
    exact ⟨imageToQuotient cfg image, quotientToImage_imageToQuotient cfg image⟩

end Zebra.Generic
