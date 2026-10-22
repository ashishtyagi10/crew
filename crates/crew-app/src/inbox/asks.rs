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
    /// A reply written in the box (`compose`), to `to`: sent in the thread.
    Send {
        thread_id: String,
        raw: String,
        to: String,
    },
    /// A forward, or a reply with files: built and sent by the worker.
    Mail(Box<super::outgoing::Outgoing>),
    /// `u`, confirmed: unsubscribe from `who`.
    Unsubscribe {
        how: super::unsub::How,
        who: String,
    },
    /// The open thread's invite (`ics::cal`): on your Google Calendar, and
    /// your answer there? Nothing comes back without the Calendar scope.
    OnCalendar(String),
    /// `c` once Google Calendar answered: the invite onto your calendar.
    AddEvent(String),
    /// `y` and an answer, as `me`: on Google Calendar, or by mail.
    Answer {
        ics: String,
        answer: super::ics::reply::Answer,
        me: String,
    },
}

impl Ask {
    /// Whether the pane waits on this as a load (`busy`), which holds the
    /// minute's check and `j`'s next page until it lands. Mail going out,
    /// pictures, saves and unsubscribes are not loads: a forward left the
    /// pane busy for good, as nothing it answers ends a load.
    pub(crate) fn loads(&self) -> bool {
        !matches!(
            self,
            Ask::Check
                | Ask::Picture { .. }
                | Ask::Save { .. }
                | Ask::Unsubscribe { .. }
                | Ask::Send { .. }
                | Ask::Mail(_)
                | Ask::OnCalendar(_)
                | Ask::AddEvent(_)
                | Ask::Answer { .. }
        )
    }
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
        /// Gmail had nothing after this page: on a first page, the whole list.
        whole: bool,
    },
    Unread(u64),
    /// A check found the mailbox as it was: nothing to list again.
    Checked,
    Opened(String, Vec<Mail>),
    /// A thread did not open (its id), and why: the reader says so where
    /// its mails would be.
    Unopened(String, String),
    /// Unsubscribed from this sender.
    Unsubscribed(String),
    /// A reply went, to these addresses.
    Sent(String),
    /// A forward went, to these addresses.
    Forwarded(String),
    /// Mail going out (a reply, a forward) did not go, and why: the one
    /// answer that unlocks the box, so another call's failure never lets a
    /// reply still on its way be sent twice.
    NotSent(String),
    /// A picture, decoded, or `None` when there is nothing to draw.
    Picture(String, Option<crate::viewpane::bitmap::Bitmap>),
    /// An attachment written to disk, or why not.
    Saved(Result<std::path::PathBuf, String>),
    /// A call failed; the pane says why and keeps what it has.
    Failed(String),
    /// Gmail refused to file thread `.0` (archive, delete, star, read), and
    /// why: the row goes back as it was.
    Refused(String, String),
    /// Google ended the grant (revoked, expired): sign in again.
    Ended(String),
    /// What Google Calendar says of the invite with this UID: your answer
    /// there, or `None` when it is not on your calendar.
    OnCalendar(String, Option<String>),
    /// An invite added to your calendar: the line to say.
    Calendar(String),
    /// An invite answered (its UID, Google's word for the answer), and the
    /// line to say.
    Answered(String, String, String),
}
