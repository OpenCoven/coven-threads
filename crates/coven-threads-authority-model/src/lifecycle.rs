//! A small, pure model of the approval lifecycle of
//! `specs/AUTOMATION-AUTHORITY-PROFILE-v1.md` §7, as `apply_lifecycle_event`
//! in `coven_threads_core::automation_authority` implements it.
//!
//! The model keeps no strings, digests, signatures or heap memory. Every
//! field of a lifecycle event that the implementation reads is replaced by
//! the *claim* it makes relative to the chain it is appended to: the
//! sequence is `Expected`, `Stale` or `Skipped`; the previous-event digest
//! is the `Head` or some `Other` digest; and so on. This keeps the state
//! space finite so [`crate::explore`] can enumerate it exhaustively and a
//! Kani harness can check it symbolically, while the differential in
//! `tests/differential.rs` concretizes each claim back into a real, signed
//! JSON event and checks that the implementation agrees.
//!
//! What is modeled: the state machine, the append-only chain (sequence,
//! previous digest, subject), the approval-evidence binding, single-use and
//! bounded recurring consumption, and the dispositions each event kind must
//! carry. What is not: signature verification, approval validity windows,
//! JSON shape beyond the fields above, and anything in `verify_dispatch`.

/// The approval's lifecycle state. `Required` is the state before the first
/// event; the four on the right are terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum State {
    /// No event yet.
    Required,
    /// `request` applied.
    Requested,
    /// `approve` applied; the approval's immutable digest is bound.
    Approved,
    /// Terminal: refused by the approver, with a no-launch disposition.
    Rejected,
    /// Terminal: the window closed, with a no-launch disposition.
    Expired,
    /// Terminal: withdrawn, with an explicit pre-launch or running disposition.
    Revoked,
    /// Terminal: a single-use approval was spent.
    Consumed,
}

impl State {
    /// Every state, in declaration order.
    pub const ALL: [State; 7] = [
        State::Required,
        State::Requested,
        State::Approved,
        State::Rejected,
        State::Expired,
        State::Revoked,
        State::Consumed,
    ];

    /// Whether no event may follow this state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            State::Rejected | State::Expired | State::Revoked | State::Consumed
        )
    }

    /// The state's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            State::Required => "required",
            State::Requested => "requested",
            State::Approved => "approved",
            State::Rejected => "rejected",
            State::Expired => "expired",
            State::Revoked => "revoked",
            State::Consumed => "consumed",
        }
    }
}

/// The event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Kind {
    /// `required -> requested`.
    Request,
    /// `requested -> approved`; introduces the approval digest.
    Approve,
    /// `requested -> rejected`.
    Reject,
    /// `requested | approved -> expired`.
    Expire,
    /// `requested | approved -> revoked`.
    Revoke,
    /// `approved -> consumed` (single use) or `approved -> approved`
    /// (recurring).
    Consume,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Kind; 6] = [
        Kind::Request,
        Kind::Approve,
        Kind::Reject,
        Kind::Expire,
        Kind::Revoke,
        Kind::Consume,
    ];

    /// The kind's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Request => "request",
            Kind::Approve => "approve",
            Kind::Reject => "reject",
            Kind::Expire => "expire",
            Kind::Revoke => "revoke",
            Kind::Consume => "consume",
        }
    }
}

/// `execution_phase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Phase {
    /// `not_applicable`
    NotApplicable,
    /// `not_started`
    NotStarted,
    /// `queued`
    Queued,
    /// `dispatching`
    Dispatching,
    /// `running`
    Running,
    /// `completed`
    Completed,
}

impl Phase {
    /// Every phase.
    pub const ALL: [Phase; 6] = [
        Phase::NotApplicable,
        Phase::NotStarted,
        Phase::Queued,
        Phase::Dispatching,
        Phase::Running,
        Phase::Completed,
    ];

    /// The phase's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Phase::NotApplicable => "not_applicable",
            Phase::NotStarted => "not_started",
            Phase::Queued => "queued",
            Phase::Dispatching => "dispatching",
            Phase::Running => "running",
            Phase::Completed => "completed",
        }
    }
}

/// `dispatch_disposition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Disposition {
    /// `not_applicable`
    NotApplicable,
    /// `launch_authorized`
    LaunchAuthorized,
    /// `cancel_before_launch`
    CancelBeforeLaunch,
    /// `request_cooperative_cancel`
    RequestCooperativeCancel,
    /// `external_effects_not_rolled_back`
    ExternalEffectsNotRolledBack,
    /// `no_launch_rejected`
    NoLaunchRejected,
    /// `no_launch_expired`
    NoLaunchExpired,
}

impl Disposition {
    /// Every disposition.
    pub const ALL: [Disposition; 7] = [
        Disposition::NotApplicable,
        Disposition::LaunchAuthorized,
        Disposition::CancelBeforeLaunch,
        Disposition::RequestCooperativeCancel,
        Disposition::ExternalEffectsNotRolledBack,
        Disposition::NoLaunchRejected,
        Disposition::NoLaunchExpired,
    ];

    /// The disposition's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Disposition::NotApplicable => "not_applicable",
            Disposition::LaunchAuthorized => "launch_authorized",
            Disposition::CancelBeforeLaunch => "cancel_before_launch",
            Disposition::RequestCooperativeCancel => "request_cooperative_cancel",
            Disposition::ExternalEffectsNotRolledBack => "external_effects_not_rolled_back",
            Disposition::NoLaunchRejected => "no_launch_rejected",
            Disposition::NoLaunchExpired => "no_launch_expired",
        }
    }
}

/// The bounded universe of per-run occurrences a consumption may bind. The
/// first [`IN_PREFIX`] fall inside the recurring approval's occurrence
/// prefix; the rest fall outside it.
pub const OCCURRENCES: u8 = 4;
/// How many occurrences lie inside the recurring occurrence prefix.
pub const IN_PREFIX: u8 = 3;
/// The largest `max_uses` the model explores. The profile allows 366; the
/// bound check is the same comparison at any value.
pub const MAX_USES_BOUND: u8 = 3;

/// A per-run binding (`consumption.occurrence_id` and `run_id`), abstracted
/// to an index in the bounded occurrence universe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Binding {
    /// Index below [`OCCURRENCES`].
    pub occurrence: u8,
}

impl Binding {
    /// Every binding in the universe.
    pub const ALL: [Binding; 4] = [
        Binding { occurrence: 0 },
        Binding { occurrence: 1 },
        Binding { occurrence: 2 },
        Binding { occurrence: 3 },
    ];

    /// Whether the index is inside the universe.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.occurrence < OCCURRENCES
    }

    /// Whether the occurrence lies inside the recurring prefix.
    #[must_use]
    pub const fn in_prefix(self) -> bool {
        self.occurrence < IN_PREFIX
    }

    /// The binding's bit in a consumed-occurrence mask.
    #[must_use]
    pub const fn bit(self) -> u8 {
        1 << self.occurrence
    }
}

/// The occurrence a single-use approval binds.
pub const SINGLE_USE_BINDING: Binding = Binding { occurrence: 0 };

/// The approval's `use`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Use {
    /// `{kind: "single_use"}`: binds exactly [`SINGLE_USE_BINDING`].
    SingleUse,
    /// `{kind: "recurring", max_uses, occurrence_prefix}`.
    Recurring {
        /// `max_uses`, at least 1.
        max_uses: u8,
    },
}

impl Use {
    /// Whether the use is well formed within the model's bounds.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        match self {
            Use::SingleUse => true,
            Use::Recurring { max_uses } => max_uses >= 1 && max_uses <= MAX_USES_BOUND,
        }
    }
}

/// The immutable approval evidence the chain may be given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Approval {
    /// Its `use`.
    pub use_: Use,
}

/// The event's `sequence` relative to the chain head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum SequenceClaim {
    /// `head.sequence + 1`.
    Expected,
    /// `head.sequence`, which is `0` and malformed before the first event.
    Stale,
    /// `head.sequence + 2`.
    Skipped,
}

/// The event's `previous_event_digest` relative to the chain head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum PreviousClaim {
    /// The head's digest, or `null` before the first event.
    Head,
    /// Some other digest.
    Other,
}

/// The event's subject (`approval_id`, `request_digest`, `decision_digest`)
/// relative to the chain's and the approval's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum SubjectClaim {
    /// The same subject.
    Same,
    /// A different approval id.
    Different,
}

/// The event's `approval_digest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum DigestClaim {
    /// `null`.
    Null,
    /// The digest of the approval evidence.
    Matching,
    /// Some other digest.
    Other,
}

/// The kind of a no-launch `occurrence_disposition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum NoLaunch {
    /// `rejected_no_launch`
    Rejected,
    /// `expired_no_launch`
    Expired,
}

/// A no-launch `occurrence_disposition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct OccurrenceDisposition {
    /// Its kind.
    pub kind: NoLaunch,
    /// Whether its occurrence and run are the approval's own.
    pub matches_approval: bool,
}

/// One lifecycle event, as the claims it makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Event {
    /// `event`
    pub kind: Kind,
    /// `from_state`, as claimed.
    pub from: State,
    /// `to_state`, as claimed.
    pub to: State,
    /// `sequence`
    pub sequence: SequenceClaim,
    /// `previous_event_digest`
    pub previous: PreviousClaim,
    /// `approval_id`, `request_digest`, `decision_digest`
    pub subject: SubjectClaim,
    /// `approval_digest`
    pub digest: DigestClaim,
    /// Whether approval evidence accompanies the event.
    pub approval_supplied: bool,
    /// `execution_phase`
    pub phase: Phase,
    /// `dispatch_disposition`
    pub disposition: Disposition,
    /// `consumption`
    pub consumption: Option<Binding>,
    /// `occurrence_disposition`
    pub occurrence_disposition: Option<OccurrenceDisposition>,
}

/// Where a chain stands. [`Chain::EMPTY`] is the chain before any event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct Chain {
    /// The current state.
    pub state: State,
    /// The last event's sequence, `0` before any event.
    pub sequence: u8,
    /// How many `consume` events the chain holds.
    pub count: u8,
    /// The consumed occurrences, one bit per [`Binding`].
    pub consumed: u8,
    /// Whether an `approve` event has bound the approval digest.
    pub approved: bool,
}

impl Chain {
    /// The chain before any event.
    pub const EMPTY: Chain = Chain {
        state: State::Required,
        sequence: 0,
        count: 0,
        consumed: 0,
        approved: false,
    };
}

/// Why the model refuses an event. A refusal may carry several reasons: the
/// model reports every violated rule, and the differential checks that the
/// implementation's single first error code names one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reason {
    /// Malformed for its kind: a disallowed `to_state`, a disposition or
    /// binding the kind may not carry, or a sequence below one.
    Shape,
    /// `approve`, or anything claimed from `approved`, without approval
    /// evidence.
    ApprovalMissing,
    /// The event binds a different approval than the evidence.
    ApprovalMismatch,
    /// A pre-approval event claims an approval digest.
    ApprovalPremature,
    /// A single-use no-launch disposition names another occurrence.
    OccurrenceMismatch,
    /// The sequence is stale or skipped.
    Replay,
    /// The event changes the chain's subject.
    SubjectMismatch,
    /// `from_state` is not the current state.
    StateForged,
    /// `previous_event_digest` is not the head.
    ChainMismatch,
    /// No such transition.
    TransitionInvalid,
    /// The binding, or a single-use approval, was already consumed.
    ConsumptionReplayed,
    /// A single-use consumption binds another operation.
    ConsumptionMismatch,
    /// A recurring consumption is outside the occurrence prefix.
    OccurrenceOutOfScope,
    /// The recurring usage bound is exceeded.
    UsageExhausted,
}

impl Reason {
    /// Every reason.
    pub const ALL: [Reason; 14] = [
        Reason::Shape,
        Reason::ApprovalMissing,
        Reason::ApprovalMismatch,
        Reason::ApprovalPremature,
        Reason::OccurrenceMismatch,
        Reason::Replay,
        Reason::SubjectMismatch,
        Reason::StateForged,
        Reason::ChainMismatch,
        Reason::TransitionInvalid,
        Reason::ConsumptionReplayed,
        Reason::ConsumptionMismatch,
        Reason::OccurrenceOutOfScope,
        Reason::UsageExhausted,
    ];

    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// A set of [`Reason`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Reasons(u16);

impl Reasons {
    /// No reason.
    pub const EMPTY: Reasons = Reasons(0);

    /// Exactly one reason.
    #[must_use]
    pub const fn single(reason: Reason) -> Reasons {
        Reasons(reason.bit())
    }

    /// Adds a reason.
    pub fn insert(&mut self, reason: Reason) {
        self.0 |= reason.bit();
    }

    /// Whether the set holds `reason`.
    #[must_use]
    pub const fn contains(self, reason: Reason) -> bool {
        self.0 & reason.bit() != 0
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The reasons, in declaration order.
    pub fn iter(self) -> impl Iterator<Item = Reason> {
        Reason::ALL.into_iter().filter(move |r| self.contains(*r))
    }
}

/// Which transition relation to run: the faithful one, or one with a
/// deliberately broken rule, so the explorer and harness can be shown to
/// find a counterexample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum Variant {
    /// §7 as written and as the implementation stands.
    Faithful,
    /// Profile erratum 1 (2026-10-20): a `consume` was judged by its target
    /// state alone, so a consume from `revoked`, `expired`, `rejected` or
    /// `requested` was accepted.
    Erratum1ConsumeChecksTargetOnly,
    /// The `single_use && consumption_count > 0` guard is dropped.
    SingleUseCountNotChecked,
    /// The recurring bound is checked one use too late (`count > max + 1`).
    RecurringBoundOffByOne,
    /// Terminal states are not absorbing: `revoke` is also accepted from
    /// `consumed`.
    RevokeAfterConsumed,
}

impl Variant {
    /// Every variant.
    pub const ALL: [Variant; 5] = [
        Variant::Faithful,
        Variant::Erratum1ConsumeChecksTargetOnly,
        Variant::SingleUseCountNotChecked,
        Variant::RecurringBoundOffByOne,
        Variant::RevokeAfterConsumed,
    ];
}

/// Whether the event is well formed for its kind (`validateLifecycleEvent`
/// without the signature, timestamp and string checks the model abstracts
/// away). `chain` matters only for a `Stale` sequence before the first
/// event, which concretizes to `0` and fails the integer minimum.
#[must_use]
pub fn shape_ok(chain: Chain, event: Event) -> bool {
    if event.to == State::Required {
        return false;
    }
    if event.sequence == SequenceClaim::Stale && chain.sequence == 0 {
        return false;
    }
    if let Some(binding) = event.consumption {
        if !binding.is_valid() || event.kind != Kind::Consume {
            return false;
        }
    }
    if event.occurrence_disposition.is_some() && !matches!(event.kind, Kind::Reject | Kind::Expire)
    {
        return false;
    }
    let disposition_kind = event.occurrence_disposition.map(|d| d.kind);
    match event.kind {
        Kind::Reject => {
            event.phase == Phase::NotStarted
                && event.disposition == Disposition::NoLaunchRejected
                && disposition_kind == Some(NoLaunch::Rejected)
        }
        Kind::Expire => {
            event.phase == Phase::NotStarted
                && event.disposition == Disposition::NoLaunchExpired
                && disposition_kind == Some(NoLaunch::Expired)
        }
        Kind::Revoke => match event.phase {
            Phase::NotStarted | Phase::Queued | Phase::Dispatching => {
                event.disposition == Disposition::CancelBeforeLaunch
            }
            Phase::Running => matches!(
                event.disposition,
                Disposition::RequestCooperativeCancel | Disposition::ExternalEffectsNotRolledBack
            ),
            // §7 constrains revocation before launch and while running; the
            // profile leaves `not_applicable` and `completed` unconstrained.
            Phase::NotApplicable | Phase::Completed => true,
        },
        Kind::Consume => {
            event.consumption.is_some()
                && event.phase == Phase::Dispatching
                && event.disposition == Disposition::LaunchAuthorized
        }
        Kind::Request | Kind::Approve => {
            event.phase == Phase::NotApplicable
                && event.disposition == Disposition::NotApplicable
                && event.consumption.is_none()
                && event.occurrence_disposition.is_none()
        }
    }
}

/// The transition table other than `consume`.
const TRANSITIONS: [(State, Kind, State); 7] = [
    (State::Required, Kind::Request, State::Requested),
    (State::Requested, Kind::Approve, State::Approved),
    (State::Requested, Kind::Reject, State::Rejected),
    (State::Requested, Kind::Expire, State::Expired),
    (State::Requested, Kind::Revoke, State::Revoked),
    (State::Approved, Kind::Revoke, State::Revoked),
    (State::Approved, Kind::Expire, State::Expired),
];

/// Where a `consume` lands for `use_`.
#[must_use]
pub const fn consume_target(use_: Use) -> State {
    match use_ {
        Use::SingleUse => State::Consumed,
        Use::Recurring { .. } => State::Approved,
    }
}

/// Appends `event` to `chain`, given `approval` when
/// `event.approval_supplied`, under `variant`.
///
/// # Errors
///
/// Every violated rule, as [`Reasons`]. A malformed event is refused with
/// [`Reason::Shape`] alone, as the implementation validates shape before it
/// reads the chain.
pub fn apply(
    variant: Variant,
    chain: Chain,
    approval: Approval,
    event: Event,
) -> Result<Chain, Reasons> {
    if !shape_ok(chain, event) {
        return Err(Reasons::single(Reason::Shape));
    }
    let evidence = event.approval_supplied.then_some(approval);
    let mut reasons = Reasons::EMPTY;

    // Approval evidence: an approve, or anything claimed from approved,
    // must carry and exactly bind it; nothing earlier may claim a digest.
    let needs_approval = event.kind == Kind::Approve || event.from == State::Approved;
    if needs_approval {
        match evidence {
            None => reasons.insert(Reason::ApprovalMissing),
            Some(_) => {
                if event.digest != DigestClaim::Matching || event.subject == SubjectClaim::Different
                {
                    reasons.insert(Reason::ApprovalMismatch);
                }
            }
        }
    } else if event.digest != DigestClaim::Null {
        reasons.insert(Reason::ApprovalPremature);
    }

    // A single-use no-launch disposition must name the approved occurrence.
    if matches!(event.kind, Kind::Reject | Kind::Expire) {
        if let Some(Approval {
            use_: Use::SingleUse,
        }) = evidence
        {
            if let Some(disposition) = event.occurrence_disposition {
                if !disposition.matches_approval {
                    reasons.insert(Reason::OccurrenceMismatch);
                }
            }
        }
    }

    // The append-only chain.
    if event.sequence != SequenceClaim::Expected {
        reasons.insert(Reason::Replay);
    }
    if chain.sequence > 0 && event.subject == SubjectClaim::Different {
        reasons.insert(Reason::SubjectMismatch);
    }
    if event.from != chain.state {
        reasons.insert(Reason::StateForged);
    }
    if event.previous != PreviousClaim::Head {
        reasons.insert(Reason::ChainMismatch);
    }

    // The transition relation.
    let allowed = match event.kind {
        Kind::Consume => match evidence {
            // Only an approval can pick the consume's target; without one
            // the implementation has already refused above.
            None => false,
            Some(approval) => {
                let target = consume_target(approval.use_);
                match variant {
                    Variant::Erratum1ConsumeChecksTargetOnly => event.to == target,
                    _ => event.from == State::Approved && event.to == target,
                }
            }
        },
        kind => {
            TRANSITIONS
                .iter()
                .any(|(from, via, to)| *from == event.from && *via == kind && *to == event.to)
                || (variant == Variant::RevokeAfterConsumed
                    && kind == Kind::Revoke
                    && event.from == State::Consumed
                    && event.to == State::Revoked)
        }
    };
    if !allowed {
        reasons.insert(Reason::TransitionInvalid);
    }

    // Consumption accounting, which the implementation reaches only for an
    // allowed consume.
    let mut count = chain.count;
    let mut consumed = chain.consumed;
    if event.kind == Kind::Consume && allowed {
        let binding = event.consumption.expect("shape requires a binding");
        let approval = evidence.expect("an allowed consume has evidence");
        count += 1;
        if consumed & binding.bit() != 0 {
            reasons.insert(Reason::ConsumptionReplayed);
        }
        match approval.use_ {
            Use::SingleUse => {
                if chain.count > 0 && variant != Variant::SingleUseCountNotChecked {
                    reasons.insert(Reason::ConsumptionReplayed);
                }
                if binding != SINGLE_USE_BINDING {
                    reasons.insert(Reason::ConsumptionMismatch);
                }
            }
            Use::Recurring { max_uses } => {
                if !binding.in_prefix() {
                    reasons.insert(Reason::OccurrenceOutOfScope);
                }
                let bound = match variant {
                    Variant::RecurringBoundOffByOne => max_uses + 1,
                    _ => max_uses,
                };
                if count > bound {
                    reasons.insert(Reason::UsageExhausted);
                }
            }
        }
        consumed |= binding.bit();
    }

    if !reasons.is_empty() {
        return Err(reasons);
    }
    Ok(Chain {
        state: event.to,
        sequence: chain.sequence + 1,
        count,
        consumed,
        approved: chain.approved || event.kind == Kind::Approve,
    })
}

/// The inductive invariant every reachable chain satisfies under
/// [`Variant::Faithful`] for `approval`. [`crate::explore`] checks it on
/// every reachable state; the Kani harness assumes it of an arbitrary chain
/// and asserts it of the successor.
#[must_use]
pub fn invariant(approval: Approval, chain: Chain) -> bool {
    let popcount = u8::try_from(chain.consumed.count_ones()).unwrap_or(u8::MAX);
    let prefix_mask: u8 = (1 << IN_PREFIX) - 1;
    let structural = (chain.sequence == 0) == (chain.state == State::Required)
        && popcount == chain.count
        && (chain.approved
            || (chain.count == 0
                && matches!(
                    chain.state,
                    State::Required
                        | State::Requested
                        | State::Rejected
                        | State::Expired
                        | State::Revoked
                )))
        && (!chain.approved
            || matches!(
                chain.state,
                State::Approved | State::Consumed | State::Revoked | State::Expired
            ));
    let by_use = match approval.use_ {
        Use::SingleUse => {
            chain.count <= 1
                && (chain.state == State::Consumed) == (chain.count == 1)
                && (chain.count == 0 || chain.consumed == SINGLE_USE_BINDING.bit())
        }
        Use::Recurring { max_uses } => {
            chain.count <= max_uses
                && chain.consumed & !prefix_mask == 0
                && chain.state != State::Consumed
        }
    };
    structural && by_use
}

/// The transition-level safety properties, checked of one accepted step
/// `chain --event--> next`. Returns the first violated property's name.
#[must_use]
pub fn step_violation(
    approval: Approval,
    chain: Chain,
    event: Event,
    next: Chain,
) -> Option<&'static str> {
    if chain.state.is_terminal() {
        return Some("terminal states are absorbing");
    }
    if next.sequence != chain.sequence + 1 {
        return Some("sequence advances by exactly one");
    }
    if chain.sequence > 0 && event.subject != SubjectClaim::Same {
        return Some("the subject is immutable");
    }
    let digest_expected = chain.approved || event.kind == Kind::Approve;
    if (event.digest == DigestClaim::Matching) != digest_expected {
        return Some("the approval digest is bound by approve and repeated after it");
    }
    if event.kind == Kind::Consume && chain.state != State::Approved {
        return Some("consumption is valid only from approved");
    }
    if event.kind == Kind::Consume && !event.approval_supplied {
        return Some("consumption carries approval evidence");
    }
    if matches!(event.kind, Kind::Reject | Kind::Expire | Kind::Revoke)
        && !matches!(chain.state, State::Requested | State::Approved)
    {
        return Some("withdrawal happens only from requested or approved");
    }
    if event.kind == Kind::Reject && chain.state != State::Requested {
        return Some("rejection happens only from requested");
    }
    if !invariant(approval, next) {
        return Some("the reachable-state invariant is preserved");
    }
    None
}
