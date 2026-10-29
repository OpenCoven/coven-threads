//! The at-most-once layers around the lifecycle: request adoption
//! (`adopt_authorization_request`) and decision consumption
//! (`consume_decision`), §6 and §9 of the profile. Each is a one-bit store
//! with a replay rule, and consumption adds the rule that a proposal-only
//! or reject decision is never consumable.

/// A decision's `outcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Outcome {
    /// `permit`
    Permit,
    /// `requires_approval`
    RequiresApproval,
    /// `degrade_to_proposal`
    DegradeToProposal,
    /// `reject`
    Reject,
}

impl Outcome {
    /// Every outcome.
    pub const ALL: [Outcome; 4] = [
        Outcome::Permit,
        Outcome::RequiresApproval,
        Outcome::DegradeToProposal,
        Outcome::Reject,
    ];

    /// The outcome's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Outcome::Permit => "permit",
            Outcome::RequiresApproval => "requires_approval",
            Outcome::DegradeToProposal => "degrade_to_proposal",
            Outcome::Reject => "reject",
        }
    }

    /// Whether a decision with this outcome may ever authorize dispatch.
    #[must_use]
    pub const fn is_consumable(self) -> bool {
        matches!(self, Outcome::Permit | Outcome::RequiresApproval)
    }
}

/// Why a consumption or adoption is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// `decision_not_consumable`: a proposal-only or reject decision.
    NotConsumable,
    /// `decision_replayed` or `request_replayed`.
    Replayed,
}

/// One decision's consumption record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Consumption {
    /// Whether the decision has been consumed.
    pub consumed: bool,
}

/// `consumeDecision` for one decision.
///
/// # Errors
///
/// [`Refusal::NotConsumable`] before [`Refusal::Replayed`], as the
/// implementation orders them.
pub fn consume(store: Consumption, outcome: Outcome) -> Result<Consumption, Refusal> {
    if !outcome.is_consumable() {
        return Err(Refusal::NotConsumable);
    }
    if store.consumed {
        return Err(Refusal::Replayed);
    }
    Ok(Consumption { consumed: true })
}

/// One request's adoption record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Adoption {
    /// Whether the request has been adopted.
    pub adopted: bool,
}

/// `adoptAuthorizationRequest` for one request, inside its validity window.
///
/// # Errors
///
/// [`Refusal::Replayed`] on a second adoption.
pub fn adopt(store: Adoption) -> Result<Adoption, Refusal> {
    if store.adopted {
        return Err(Refusal::Replayed);
    }
    Ok(Adoption { adopted: true })
}
