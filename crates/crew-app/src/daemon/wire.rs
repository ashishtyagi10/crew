//! One request in, one reply out — the daemon's whole wire face.
//!
//! Split from [`super`] so the resident's STATE and the protocol that exposes it are separate
//! files: every new request is an arm here, and none of them is a reason to reopen the struct.
use crate::ipc_types::{Reply, Request};

use super::{intent, Daemon};

/// Answer one request. `None` for anything the daemon does not serve — the ask ops belong to the
/// GUI's endpoint, and a client that dials the wrong socket must get silence rather than a
/// confidently wrong reply.
pub(crate) fn answer(req: &Request, d: &mut Daemon) -> Option<Reply> {
    match req {
        Request::DaemonStatus { .. } => {
            let st = d.status();
            Some(Reply::Daemon {
                pid: st.pid,
                uptime_s: st.uptime_s,
                sessions: st.sessions,
                version: st.version,
            })
        }
        Request::OpenSession { label, cwd, .. } => {
            let dir = cwd.as_deref().map(std::path::Path::new);
            Some(match d.sessions.open(label, dir) {
                Ok(id) => Reply::Session { id },
                Err(e) => Reply::Failed {
                    message: format!("could not start a session: {e}"),
                },
            })
        }
        Request::Sessions { .. } => Some(Reply::Sessions {
            sessions: d
                .sessions
                .cards()
                .into_iter()
                .map(|c| crate::ipc_types::SessionCard {
                    id: c.id,
                    label: c.label,
                    cwd: c.cwd,
                    alive: c.alive,
                })
                .collect(),
        }),
        Request::Channels { .. } => Some(Reply::Channels {
            registered: d.channels.kinds().into_iter().map(str::to_string).collect(),
            ready: d
                .channels
                .ready_kinds()
                .into_iter()
                .map(str::to_string)
                .collect(),
        }),
        Request::Say { to, text, .. } => Some(match d.channels.send(to, text) {
            Ok(()) => Reply::Sent {
                id: to.clone(),
                delivered: true,
            },
            Err(e) => Reply::Failed { message: e },
        }),
        Request::SessionSend { id, line, .. } => Some(match d.sessions.send(id, line) {
            Some(delivered) => Reply::Sent {
                id: id.clone(),
                delivered,
            },
            None => Reply::Failed {
                message: format!("no such session: {id}"),
            },
        }),
        Request::SessionPoll { id, after, .. } => Some(match d.sessions.output(id, *after) {
            Some((lines, next, dropped)) => Reply::Events {
                lines,
                next,
                dropped,
            },
            None => Reply::Failed {
                message: format!("no such session: {id}"),
            },
        }),
        Request::Watch {
            text,
            to,
            fire_ms,
            repeat_secs,
            ..
        } => {
            let repeat = match repeat_secs {
                Some(secs) => intent::Repeat::Every { secs: *secs },
                None => intent::Repeat::Once,
            };
            // An empty address means "wherever you can answer me", which only the daemon knows.
            // With no answer for that, the intent is refused rather than stored: an alarm that
            // fires into nowhere is worse than one that was never set.
            let to = &match to.is_empty() {
                false => to.clone(),
                true => match d.channels.default_address() {
                    Some(a) => a,
                    None => {
                        return Some(Reply::Failed {
                            message: "nowhere to answer \u{2014} name a channel address with \
                                      --to, like --to telegram:12345"
                                .to_string(),
                        })
                    }
                },
            };
            Some(
                match d
                    .watch
                    .add(text, to, *fire_ms, repeat, crate::chattime::unix_now_ms())
                {
                    Ok(it) => Reply::Watched {
                        id: it.id,
                        fire_ms: it.fire_ms,
                    },
                    Err(e) => Reply::Failed {
                        message: format!("could not write the watchlist: {e}"),
                    },
                },
            )
        }
        Request::Watching { .. } => Some(Reply::Watchlist {
            intents: d
                .watch
                .live()
                .into_iter()
                .map(|i| crate::ipc_types::IntentCard {
                    id: i.id,
                    text: i.text,
                    to: i.to,
                    fire_ms: i.fire_ms,
                    repeat: i.repeat.label(),
                    created_ms: i.created_ms,
                })
                .collect(),
        }),
        Request::Unwatch { id, .. } => {
            Some(match d.watch.cancel(id, crate::chattime::unix_now_ms()) {
                Ok(found) => Reply::Unwatched {
                    id: id.clone(),
                    found,
                },
                Err(e) => Reply::Failed {
                    message: format!("could not write the watchlist: {e}"),
                },
            })
        }
        Request::Snooze { id, delay_ms, .. } => Some(
            match d
                .watch
                .snooze(id, *delay_ms, crate::chattime::unix_now_ms())
            {
                Ok(fire_ms) => Reply::Snoozed {
                    id: id.clone(),
                    fire_ms,
                },
                Err(e) => Reply::Failed {
                    message: format!("could not write the watchlist: {e}"),
                },
            },
        ),
        Request::CloseSession { id, .. } => Some(match d.sessions.close(id) {
            Some(was_alive) => Reply::Closed {
                id: id.clone(),
                was_alive,
            },
            None => Reply::Failed {
                message: format!("no such session: {id}"),
            },
        }),
        _ => None,
    }
}
