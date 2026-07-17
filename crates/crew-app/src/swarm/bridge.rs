//! Off-thread scheduler bridge: runs `crew_hive::Scheduler` on a dedicated
//! worker thread with its own tokio current-thread runtime, forwarding
//! `HiveEvent`s to a std::sync::mpsc channel for frame-by-frame draining.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use crew_hive::{
    budget_governor, AgentFactory, Blackboard, Budget, EventBus, Fleet, HiveEvent, Scheduler,
    TaskGraph,
};

/// Forward every bus event into the mpsc sender until the bus closes or the
/// receiver is gone. A subscriber that falls behind the bus's 256-slot
/// buffer gets `Lagged` — skip and keep going (the old `while let Ok` loop
/// broke there, silently freezing telemetry for the rest of the run).
pub(super) async fn forward(
    mut sub: tokio::sync::broadcast::Receiver<HiveEvent>,
    tx: mpsc::Sender<HiveEvent>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match sub.recv().await {
            Ok(ev) => {
                if tx.send(ev).is_err() {
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
}

/// Handle to a running swarm engine. Cheaply drains events each frame.
pub struct SwarmHandle {
    rx: Receiver<HiveEvent>,
    cancel: Arc<AtomicBool>,
    graph: TaskGraph,
}

impl SwarmHandle {
    /// Spawn the scheduler on a worker thread and return a handle.
    ///
    /// The worker thread owns a `tokio` current-thread runtime; its `EventBus`
    /// is drained into the mpsc channel so the UI thread never blocks. When
    /// `budget` is `Some`, a [`budget_governor`] runs alongside the scheduler on
    /// the same cancel flag and stops the fleet once fleet cost exceeds the cap.
    pub fn spawn(
        graph: TaskGraph,
        factory: Arc<dyn AgentFactory>,
        concurrency: usize,
        budget: Option<Budget>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<HiveEvent>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_sched = cancel.clone();
        let cancel_gov = cancel.clone();
        let graph_thread = graph.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio current-thread runtime");

            rt.block_on(async move {
                let bus = EventBus::new(EventBus::DEFAULT_CAPACITY);
                let sub = bus.subscribe();
                // Build the governor future before `bus` moves into the scheduler;
                // it gets its own clone and shares the scheduler's cancel flag.
                let governor = budget.map(|b| budget_governor(bus.clone(), b, cancel_gov));
                let board = Blackboard::new();
                let sched = Scheduler::new(graph_thread, board, bus, factory, concurrency)
                    .with_cancel(cancel_sched);

                // Drain the broadcast bus into the mpsc sender concurrently
                // with the scheduler. When sched completes, the broadcast
                // sender is dropped; recv() returns Closed and drain exits.
                let drain = forward(sub, tx);

                match governor {
                    Some(governor) => {
                        tokio::join!(sched.run(), drain, governor);
                    }
                    None => {
                        tokio::join!(sched.run(), drain);
                    }
                }
            });
        });

        Self { rx, cancel, graph }
    }

    /// Non-blocking drain of pending events into the fleet (call each frame).
    /// Returns the number of events applied, so callers can skip a redraw when
    /// nothing changed.
    pub fn drain(&self, fleet: &mut Fleet) -> usize {
        let mut n = 0;
        while let Ok(ev) = self.rx.try_recv() {
            fleet.apply(&ev);
            n += 1;
        }
        n
    }

    /// Signal the scheduler to stop spawning new tasks.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether the run has been cancelled — either explicitly via [`Self::cancel`]
    /// or by the budget governor tripping the shared flag.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// The task graph this swarm is executing.
    pub fn graph(&self) -> &TaskGraph {
        &self.graph
    }
}
