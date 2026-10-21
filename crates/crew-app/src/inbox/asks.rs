//! What the pane asks the worker, and what comes back (`worker`).
use super::api::Mail;
use super::wire::Thread;

pub(crate) enum Ask {
    /// The first page for `query` (empty = the inbox).
    List(String),
    /// The page after the last one.
    More,
    Open(String),
    /// Has anything changed? Re-list when the mailbox moved.
    Check,
    /// Save `raw` (RFC 2822, base64url) as a draft in the thread.
    Draft {
        thread_id: String,
        raw: String,
        subject: String,
    },
    /// Add and take away labels on a thread (archive, read, star).
    Relabel {
        id: String,
        add: Vec<&'static str>,
        remove: Vec<&'static str>,
    },
    /// Move a thread to Trash, or (`back`) out of it.
    Trash {
        id: String,
        back: bool,
    },
    /// A picture the mail carries: inline `data`, or attachment `att` of
    /// message `msg`, decoded here and sent back under `key`.
    Picture {
        key: String,
        msg: String,
        att: Option<String>,
        data: Option<String>,
    },
    /// Save attachment `att` (or inline `data`) of message `msg` to
    /// Downloads as `name`.
    Save {
        msg: String,
        att: Option<String>,
        data: Option<String>,
        name: String,
    },
    /// `u`, confirmed: unsubscribe from `who`.
    Unsubscribe {
        how: super::unsub::How,
        who: String,
    },
}

pub(crate) enum Got {
    /// The signed-in account.
    Account(String),
    Page {
        query: String,
        rows: Vec<Thread>,
        next: Option<String>,
        /// Rows to add to the list rather than replace it.
        more: bool,
    },
    Unread(u64),
    /// A check found the mailbox as it was: nothing to list again.
    Checked,
    Opened(String, Vec<Mail>),
    /// A draft was saved; its subject.
    Drafted(String),
    /// Unsubscribed from this sender.
    Unsubscribed(String),
    /// A picture, decoded, or `None` when there is nothing to draw.
    Picture(String, Option<crate::viewpane::bitmap::Bitmap>),
    /// An attachment written to disk, or why not.
    Saved(Result<std::path::PathBuf, String>),
    /// A call failed; the pane says why and keeps what it has.
    Failed(String),
    /// Google ended the grant (revoked, expired): sign in again.
    Ended(String),
}
