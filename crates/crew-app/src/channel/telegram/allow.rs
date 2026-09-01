//! Who crew will listen to on Telegram.
use std::collections::BTreeSet;

/// Who crew will listen to. An assistant with a public address is an assistant anyone can drive,
/// so silence toward strangers is the default: an empty allowlist accepts NOBODY rather than
/// everybody. The alternative — open until configured — is a window that stands open for
/// exactly as long as it takes the owner to notice.
#[derive(Debug, Default, Clone)]
pub(crate) struct Allowlist(BTreeSet<i64>);

impl Allowlist {
    /// Parse `CREW_TELEGRAM_CHATS`: comma or space separated chat ids.
    pub(crate) fn parse(raw: &str) -> Self {
        Self(
            raw.split([',', ' ', '\t'])
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect(),
        )
    }

    pub(crate) fn allows(&self, chat_id: i64) -> bool {
        self.0.contains(&chat_id)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The only chat on the list, when there is exactly one.
    pub(crate) fn sole(&self) -> Option<i64> {
        (self.0.len() == 1).then(|| *self.0.iter().next().expect("length checked"))
    }
}
