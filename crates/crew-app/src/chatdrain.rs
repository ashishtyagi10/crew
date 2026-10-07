//! Draining the broker: the per-frame poll that turns plugin events into
//! transcript cards, and resetting when a broker goes away.
//!
//! Its sibling [`crate::chattranscript`] holds what happens to the transcript
//! itself — the cap, the fold slack, and the task tally.
//!
//! Split out of [`crate::chat`] for the line cap.
use crate::chat::ChatPane;
use crate::chatevents::{classify, HostAction, PollResult};
use crew_plugin::PluginEvent;

impl ChatPane {
    /// Drop everything that belonged to one broker process: what it had
    /// running, and any plan it was holding for an answer. Called on both
    /// edges — a new broker (`Ready`) and a lost one (`Error`) — because both
    /// invalidate the same state, and neither can be relied on to arrive.
    pub(crate) fn reset_broker_state(&mut self) {
        self.running_tasks.clear();
        (self.plan_pending, self.asking) = (false, Default::default());
        self.pending_reply_usage = None; // it must not tag the next broker's reply
        self.thoughts.drop_live(); // no broker will finish it
    }

    /// End an in-flight browser sign-in, and SAY SO. The single place a live
    /// flow is dismissed while its pane is still around.
    ///
    /// Dropping the receiver is what cancels it: the worker's `send` then
    /// fails and the outcome — possibly a key OpenRouter has already minted
    /// against the user's account — is discarded. That must never be silent,
    /// or the only trace left is a key the user has to find and revoke by
    /// hand. A no-op (and no note) when no sign-in was in flight.
    pub(crate) fn cancel_oauth(&mut self) {
        if self.oauth.take().is_some() {
            self.push_note("openrouter sign-in cancelled".into());
        }
    }

    /// Drain plugin events; return PollResult with changed flag and any host actions.
    pub fn poll(&mut self) -> PollResult {
        self.track_turn();
        let mut events = self.plugin.try_recv();
        for ev in &events {
            self.heard(ev);
        }
        // A heartbeat is for the watchdog alone: it changes nothing drawn.
        events.retain(|e| !matches!(e, PluginEvent::Alive {}));
        if events.is_empty() {
            let actions: Vec<HostAction> = self.watchdog().into_iter().collect();
            return PollResult {
                changed: !actions.is_empty(),
                actions,
            };
        }
        let mut actions = Vec::new();
        for ev in events {
            if let Some(action) = classify(&ev) {
                actions.push(action);
            } else {
                match ev {
                    PluginEvent::Ready {
                        provider, channels, ..
                    } => {
                        self.connected = true;
                        // The handshake landing is the "it's alive" moment the
                        // spawn's "starting…" LOG line promised — close the loop.
                        actions.push(HostAction::Status {
                            error: false,
                            message: if provider == "crew" {
                                "agent smith connected".to_string()
                            } else {
                                format!("{provider} plugin connected")
                            },
                        });
                        // A fresh broker has nothing running and no plan
                        // waiting — and it numbers its tasks from 1 again, so
                        // anything left from the last one would both lie and
                        // collide.
                        self.reset_broker_state();
                        if self.channel.is_empty() {
                            if let Some(ch) = channels.into_iter().next() {
                                self.channel = ch;
                            }
                        }
                    }
                    PluginEvent::Roster {
                        agents,
                        provider,
                        signins,
                    } => {
                        self.agents = agents;
                        self.tools
                            .prebind(self.agents.iter().map(|a| a.name.as_str()));
                        // The broker is the authority on who serves: its pin
                        // (a sign-in, a model pick) reaches the picker here.
                        if let Some(p) = &provider {
                            crate::shellprobe::note_pin(p);
                        }
                        crate::modelsignin::set(signins, provider);
                    }
                    // The picker is the broker's WHOLE answer to the bare
                    // construct — it settles the pane like a `Message` would.
                    PluginEvent::SignOut { options } => self.open_sign_out_picker(&options),
                    PluginEvent::Task { id, running, .. } => self.absorb_task(id, running),
                    PluginEvent::Plan { pending } => self.plan_pending = pending,
                    PluginEvent::Approval { id, question, .. } => self.ask_user(id, question),
                    PluginEvent::Activity { agent, state, from } => {
                        // The broker names the agent its last hive event
                        // was about — the tool lines are keyed by that name.
                        if from == "hive" {
                            self.tools.bind(&agent);
                        }
                        self.absorb_activity(agent, &state, from);
                    }
                    PluginEvent::Stats {
                        tokens,
                        agent,
                        ms,
                        ctx,
                        tok_in,
                        tok_out,
                        cost_microusd,
                        tools,
                        ..
                    } => {
                        self.absorb_stats(tokens, agent, ms, ctx, tok_in, tok_out, cost_microusd);
                        self.note_swarm_tools(tools);
                    }
                    // Mid-reply token ticks: the header's agent-name pulse
                    // (`chatliveness`); the summary footer reads settled `ctx`.
                    PluginEvent::StatsTick { agent, .. } => {
                        self.note_tokens(&agent, crate::anim::now_ms())
                    }
                    PluginEvent::Delta { agent, text, sub } => self.absorb_delta(agent, text, sub),
                    PluginEvent::Thought { agent, text } => self.absorb_thought(agent, text),
                    PluginEvent::Steered { text, .. } => self.absorb_steered(text),
                    PluginEvent::Message {
                        sender,
                        text,
                        ts,
                        meta,
                        ..
                    } => self.absorb_message(sender, text, ts, meta),
                    PluginEvent::HivePlan { tasks } => self.absorb_hive_plan(tasks),
                    PluginEvent::Hive { event } => {
                        // Quiet lifecycle tee: the run's spawn/state beats
                        // land in the LOG (and /log) without flashing the bar.
                        if let Some((error, message)) = crate::chatswarmlog::log_line_named(
                            self.swarm.as_ref(),
                            &self.tools.names,
                            &event,
                        ) {
                            actions.push(HostAction::Log { error, message });
                        }
                        self.absorb_hive(&event);
                    }
                    // Its output ended: nothing it was running survives it, the
                    // pane says so and starts it again (`chatrevive`), and the
                    // rest of this batch is the dead broker's — dropped.
                    PluginEvent::Error { message } => {
                        actions.push(self.broker_lost(&message));
                        break;
                    }
                    _ => {}
                }
            }
        }
        // Flush check: the busy→idle transition always arrives via one of the
        // events just processed (Activity idle, a swarm fold, or a Message
        // clearing `awaiting`), so it's enough to re-check here rather than
        // on every tick. One message per turn — the next flush waits for the
        // reply to *that* send to land and settle the pane again.
        //
        // Also gated on `connected`: a broker death mid-swarm/mid-active-hop
        // folds the swarm and flushes active hops in the same `Error` arm
        // that flips `connected` false — so `is_busy()` can go false in the
        // very same drain as the disconnect. Without this gate that race
        // pops the queue and calls `send_now` against the dead child right
        // here, silently dropping the text. Requiring `connected` closes it;
        // the queue then waits for a real reconnect (a fresh `Ready`) to
        // flush.
        if self.connected && !self.is_busy() {
            if let Some(text) = self.queued.pop_front() {
                self.send_now(text);
            }
        }
        PollResult {
            changed: true,
            actions,
        }
    }
}

#[cfg(test)]
#[path = "chatdrain_tests.rs"]
mod tests;
