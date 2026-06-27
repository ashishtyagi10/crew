//! Off-thread scheduler bridge: runs `crew_hive::Scheduler` on a dedicated
//! worker thread with its own tokio current-thread runtime, forwarding
//! `HiveEvent`s to a std::sync::mpsc channel for frame-by-frame draining.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use crew_hive::{AgentFactory, Blackboard, EventBus, Fleet, HiveEvent, Scheduler, TaskGraph};

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
    /// is drained into the mpsc channel so the UI thread never blocks.
    pub fn spawn(graph: TaskGraph, factory: Arc<dyn AgentFactory>, concurrency: usize) -> Self {
        let (tx, rx) = mpsc::channel::<HiveEvent>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();
        let graph_thread = graph.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio current-thread runtime");

            rt.block_on(async move {
                let bus = EventBus::new(256);
                let mut sub = bus.subscribe();
                let board = Blackboard::new();
                let sched = Scheduler::new(graph_thread, board, bus, factory, concurrency)
                    .with_cancel(cancel_clone);

                // Drain the broadcast bus into the mpsc sender concurrently
                // with the scheduler. When sched completes, the broadcast
                // sender is dropped; sub.recv() then returns Err and drain exits.
                let drain = async move {
                    while let Ok(ev) = sub.recv().await {
                        if tx.send(ev).is_err() {
                            break;
                        }
                    }
                };

                tokio::join!(sched.run(), drain);
            });
        });

        Self { rx, cancel, graph }
    }

    /// Non-blocking drain of pending events into the fleet (call each frame).
    pub fn drain(&self, fleet: &mut Fleet) {
        while let Ok(ev) = self.rx.try_recv() {
            fleet.apply(&ev);
        }
    }

    /// Signal the scheduler to stop spawning new tasks.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The task graph this swarm is executing.
    pub fn graph(&self) -> &TaskGraph {
        &self.graph
    }
}
