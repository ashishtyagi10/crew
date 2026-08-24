//! The gate: who is allowed to fire an irreversible tool, and what happens when nobody answers.
//!
//! [`super::tier`] says whether an action can be undone. This says whether it may proceed, and
//! the two questions come apart on one axis — WHO ASKED. A person sitting in front of a pane
//! typing `run the tests` is already the approval; interrupting them to confirm their own
//! keystroke is theatre. The same tool call arriving from a phone, or from a trigger firing at
//! 3am with nobody awake, has no human behind it and must find one.
//!
//! So the rule is not "irreversible ⇒ ask". It is "irreversible AND no present human ⇒ ask, and
//! if nobody answers, DENY". Silence is never consent: an assistant that treats an unanswered
//! 3am prompt as a yes is worse than one that does nothing.
use super::tier::Tier;

/// How long an unanswered approval stays open before it is denied.
pub const DEFAULT_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Who is asking for the tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requester {
    /// A human typing into a pane on this machine. They can see the output and stop it.
    LocalPane,
    /// A message that arrived over a channel — a phone, a room, a thread. There is a human at
    /// the other end, but they are not watching the tool run, so they get asked.
    Channel(String),
    /// A schedule or a watch fired. Nobody asked for this right now, and nobody may be awake.
    Trigger(String),
}

/// The environment variable a host sets on a broker child to say who its work is for.
///
/// The gate lives in the broker process, but WHO ASKED is known only to whoever spawned it. With
/// no way to carry that across the process boundary every broker looked like a person at the
/// keyboard — including the ones a daemon starts for a phone conversation — so the gate trusted
/// a remote sender completely. This is how the answer travels.
pub const REQUESTER_ENV: &str = "CREW_REQUESTER";

impl Requester {
    /// Parse the wire form: `pane`, `channel:<address>`, `trigger:<name>`.
    ///
    /// Anything unrecognised — including an empty or malformed value — is a TRIGGER, the most
    /// restricted kind, not a pane. A typo in this variable must not be a way to be trusted.
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        match raw.split_once(':') {
            Some(("channel", rest)) if !rest.is_empty() => Requester::Channel(rest.to_string()),
            Some(("trigger", rest)) if !rest.is_empty() => Requester::Trigger(rest.to_string()),
            _ if raw == "pane" => Requester::LocalPane,
            _ if raw.is_empty() => Requester::LocalPane,
            _ => Requester::Trigger(format!("unrecognised:{raw}")),
        }
    }

    /// The wire form, for a host to hand a broker child.
    pub fn to_env(&self) -> String {
        match self {
            Requester::LocalPane => "pane".to_string(),
            Requester::Channel(c) => format!("channel:{c}"),
            Requester::Trigger(t) => format!("trigger:{t}"),
        }
    }

    /// Read it from this process's environment. Absent = a pane, which is how every broker a
    /// GUI pane spawns keeps behaving exactly as it always has.
    pub fn from_env() -> Self {
        Self::parse(&std::env::var(REQUESTER_ENV).unwrap_or_default())
    }

    /// Is a human present and watching this happen as it happens?
    pub fn is_present_human(&self) -> bool {
        matches!(self, Requester::LocalPane)
    }

    /// Where an approval question should be sent back to, if it is asked.
    pub fn reply_to(&self) -> &str {
        match self {
            Requester::LocalPane => "pane",
            Requester::Channel(c) => c,
            Requester::Trigger(_) => "",
        }
    }
}

/// What the gate decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Fire it.
    Allow,
    /// Ask first, on this address. Carries the approval id to answer with.
    Ask { id: String, reply_to: String },
    /// Refused, with a reason fit to show a human.
    Deny(String),
}

/// Tunable policy. The defaults are today's behaviour: a person at the keyboard is trusted with
/// their own machine.
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// May a present human's irreversible call fire without a second confirmation?
    pub trust_present_human: bool,
    /// How long an approval waits before it is denied.
    pub timeout_ms: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            trust_present_human: true,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

/// One approval waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub id: String,
    pub tool: String,
    pub tier: Tier,
    pub requester: Requester,
    pub asked_ms: u64,
}

/// How a pending approval ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Granted,
    Denied,
    /// Nobody answered in time. Counted separately from a deliberate denial, because "they said
    /// no" and "nobody was there" mean different things when you read the ledger later.
    TimedOut,
}

/// The gate's live state: what is waiting, and what each one was asked about.
#[derive(Default)]
pub struct Gate {
    pending: Vec<Pending>,
    next: u64,
}

impl Gate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decide one tool call. Anything below [`Tier::Irreversible`] runs; an irreversible call
    /// runs only for a present human (when policy allows), and otherwise opens an approval.
    pub fn decide(
        &mut self,
        tool: &str,
        tier: Tier,
        requester: &Requester,
        policy: Policy,
        now_ms: u64,
    ) -> Decision {
        if !tier.needs_approval() {
            return Decision::Allow;
        }
        if requester.is_present_human() && policy.trust_present_human {
            return Decision::Allow;
        }
        // A trigger has no one to ask. Refusing is the honest answer — the alternative is
        // opening a question into an empty room and calling the silence a yes.
        if requester.reply_to().is_empty() {
            return Decision::Deny(format!(
                "{tool} cannot be undone and nothing scheduled it can be asked for approval"
            ));
        }
        self.next += 1;
        let id = format!("a{}", self.next);
        self.pending.push(Pending {
            id: id.clone(),
            tool: tool.to_string(),
            tier,
            requester: requester.clone(),
            asked_ms: now_ms,
        });
        Decision::Ask {
            id,
            reply_to: requester.reply_to().to_string(),
        }
    }

    /// Answer one approval. `None` if the id is unknown — including one already answered, so a
    /// second grant cannot re-fire an action.
    pub fn answer(&mut self, id: &str, granted: bool) -> Option<(Pending, Outcome)> {
        let i = self.pending.iter().position(|p| p.id == id)?;
        let p = self.pending.remove(i);
        Some((
            p,
            if granted {
                Outcome::Granted
            } else {
                Outcome::Denied
            },
        ))
    }

    /// Retire everything that has waited longer than the policy allows. Returns them so the
    /// caller can tell each requester its action did not happen.
    pub fn expire(&mut self, now_ms: u64, policy: Policy) -> Vec<Pending> {
        let cutoff = now_ms.saturating_sub(policy.timeout_ms);
        let (expired, live): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| p.asked_ms <= cutoff);
        self.pending = live;
        expired
    }

    /// Everything still waiting.
    pub fn pending(&self) -> &[Pending] {
        &self.pending
    }
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;
