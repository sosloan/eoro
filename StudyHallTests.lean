/-
Author: Steve Sloan (Prince Sloan)
System: Lean 4
Purpose: Integration tests for the StudyHall "Learn-to-Earn" /
         "Play-to-Earn" economy axioms defined in StudyHall.lean.

Run with:  lake env lean StudyHallTests.lean
-/

import StudyHall

open StudyHall

-- ─────────────────────────────────────────────────────────────────────────────
-- §1  Constructor / decidable-equality smoke tests
-- ─────────────────────────────────────────────────────────────────────────────

section ConstructorTests

-- Every Source constructor is distinct
#eval (Source.Institution == Source.Sponsor)       -- false
#eval (Source.Philanthropy == Source.Philanthropy) -- true
#eval (Source.UserRevenue  == Source.TreasurySeed) -- false

-- Every Flow constructor is distinct
#eval (Flow.LearnToEarn == Flow.PlayToEarn)  -- false
#eval (Flow.Reserve     == Flow.Reserve)     -- true
#eval (Flow.MentorReward == Flow.LearnToEarn)-- false

-- DecidableEq instances work with native_decide
example : Source.Institution ≠ Source.Sponsor     := by decide
example : Flow.LearnToEarn   ≠ Flow.PlayToEarn    := by decide
example : Flow.Reserve       = Flow.Reserve       := by decide

end ConstructorTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §2  isRealValue — every Source must carry fiat / real-world value
-- ─────────────────────────────────────────────────────────────────────────────

section IsRealValueTests

-- All five sources are real-valued
example : isRealValue .Institution  := trivial
example : isRealValue .Sponsor      := trivial
example : isRealValue .Philanthropy := trivial
example : isRealValue .UserRevenue  := trivial
example : isRealValue .TreasurySeed := trivial

-- Universal statement (mirrors the second conjunct of sustainability_truth)
example : ∀ s : Source, isRealValue s := fun s => by cases s <;> trivial

end IsRealValueTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §3  funded — valid Flow × Source pairings
-- ─────────────────────────────────────────────────────────────────────────────

section FundedValidTests

-- LearnToEarn accepts every Source
example : funded .LearnToEarn .Institution  := trivial
example : funded .LearnToEarn .Sponsor      := trivial
example : funded .LearnToEarn .Philanthropy := trivial
example : funded .LearnToEarn .UserRevenue  := trivial
example : funded .LearnToEarn .TreasurySeed := trivial

-- PlayToEarn accepts Sponsor, UserRevenue, TreasurySeed
example : funded .PlayToEarn .Sponsor      := trivial
example : funded .PlayToEarn .UserRevenue  := trivial
example : funded .PlayToEarn .TreasurySeed := trivial

-- MentorReward accepts Institution and UserRevenue
example : funded .MentorReward .Institution := trivial
example : funded .MentorReward .UserRevenue := trivial

-- Reserve is funded by *any* source
example : funded .Reserve .Institution  := trivial
example : funded .Reserve .Sponsor      := trivial
example : funded .Reserve .Philanthropy := trivial
example : funded .Reserve .UserRevenue  := trivial
example : funded .Reserve .TreasurySeed := trivial

end FundedValidTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §4  funded — invalid / boundary pairings must be False
-- ─────────────────────────────────────────────────────────────────────────────

section FundedInvalidTests

-- PlayToEarn is NOT funded by pure Philanthropy or Institution
example : ¬ funded .PlayToEarn .Philanthropy := by decide
example : ¬ funded .PlayToEarn .Institution  := by decide

-- MentorReward is NOT funded by Sponsor, Philanthropy, or TreasurySeed
example : ¬ funded .MentorReward .Sponsor      := by decide
example : ¬ funded .MentorReward .Philanthropy := by decide
example : ¬ funded .MentorReward .TreasurySeed := by decide

end FundedInvalidTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §5  Funding completeness — every Flow has at least one valid Source
-- ─────────────────────────────────────────────────────────────────────────────

section FundingCompletenessTests

-- Verified via the first conjunct of sustainability_truth
example : ∀ f : Flow, ∃ s : Source, funded f s :=
  sustainability_truth.1

-- Individual witnesses
example : ∃ s, funded .LearnToEarn  s := ⟨.Institution, trivial⟩
example : ∃ s, funded .PlayToEarn   s := ⟨.Sponsor,     trivial⟩
example : ∃ s, funded .MentorReward s := ⟨.Institution, trivial⟩
example : ∃ s, funded .Reserve      s := ⟨.TreasurySeed, trivial⟩

end FundingCompletenessTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §6  SustainableSystem end-to-end integration test
-- ─────────────────────────────────────────────────────────────────────────────

section SustainableSystemTests

-- The top-level sustainability theorem type-checks and holds
#check @sustainability_truth
example : SustainableSystem := sustainability_truth

-- First pillar: funding exists for every flow
example : ∀ f : Flow, ∃ s : Source, funded f s :=
  sustainability_truth.1

-- Second pillar: all sources carry real value
example : ∀ s : Source, isRealValue s :=
  sustainability_truth.2

end SustainableSystemTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §7  Repr round-trip tests (observability / logging)
-- ─────────────────────────────────────────────────────────────────────────────

section ReprTests

#eval reprStr Source.Institution   -- "StudyHall.Source.Institution"
#eval reprStr Source.TreasurySeed  -- "StudyHall.Source.TreasurySeed"
#eval reprStr Flow.LearnToEarn     -- "StudyHall.Flow.LearnToEarn"
#eval reprStr Flow.Reserve         -- "StudyHall.Flow.Reserve"

end ReprTests

-- ─────────────────────────────────────────────────────────────────────────────
-- §8  Decidability exhaustive check (all 4 × 5 = 20 pairs)
-- ─────────────────────────────────────────────────────────────────────────────

section ExhaustiveDecidabilityTests

-- Each cell must be decidable — native_decide closes all goals
example : decide (funded .LearnToEarn  .Institution)  = true  := rfl
example : decide (funded .LearnToEarn  .Sponsor)      = true  := rfl
example : decide (funded .LearnToEarn  .Philanthropy) = true  := rfl
example : decide (funded .LearnToEarn  .UserRevenue)  = true  := rfl
example : decide (funded .LearnToEarn  .TreasurySeed) = true  := rfl

example : decide (funded .PlayToEarn   .Institution)  = false := rfl
example : decide (funded .PlayToEarn   .Sponsor)      = true  := rfl
example : decide (funded .PlayToEarn   .Philanthropy) = false := rfl
example : decide (funded .PlayToEarn   .UserRevenue)  = true  := rfl
example : decide (funded .PlayToEarn   .TreasurySeed) = true  := rfl

example : decide (funded .MentorReward .Institution)  = true  := rfl
example : decide (funded .MentorReward .Sponsor)      = false := rfl
example : decide (funded .MentorReward .Philanthropy) = false := rfl
example : decide (funded .MentorReward .UserRevenue)  = true  := rfl
example : decide (funded .MentorReward .TreasurySeed) = false := rfl

example : decide (funded .Reserve      .Institution)  = true  := rfl
example : decide (funded .Reserve      .Sponsor)      = true  := rfl
example : decide (funded .Reserve      .Philanthropy) = true  := rfl
example : decide (funded .Reserve      .UserRevenue)  = true  := rfl
example : decide (funded .Reserve      .TreasurySeed) = true  := rfl

end ExhaustiveDecidabilityTests
