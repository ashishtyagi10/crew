//! The Gmail calls that file a thread for the pane's keys (`triage`): its
//! labels (archive, read, star) and Trash (delete). Both sit inside
//! `gmail.modify`; deleting for good would need the full-mail scope.
use super::api::Gmail;
use super::query::enc;

impl Gmail {
    /// Add and take away labels on a whole thread: archive is taking `INBOX`,
    /// read is taking `UNREAD`, a star is adding `STARRED`.
    pub(crate) fn relabel(
        &mut self,
        id: &str,
        add: &[&str],
        remove: &[&str],
    ) -> Result<(), String> {
        let body = serde_json::json!({"addLabelIds": add, "removeLabelIds": remove});
        self.post(&format!("/threads/{}/modify", enc(id)), body)
            .map(|_| ())
    }

    /// Gmail's delete: the thread to Trash, or (`back`) out of it again.
    pub(crate) fn trash(&mut self, id: &str, back: bool) -> Result<(), String> {
        let verb = if back { "untrash" } else { "trash" };
        self.post(
            &format!("/threads/{}/{verb}", enc(id)),
            serde_json::json!({}),
        )
        .map(|_| ())
    }
}
