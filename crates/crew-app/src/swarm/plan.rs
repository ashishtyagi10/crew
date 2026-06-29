//! Goal → `TaskGraph` on a short-lived worker thread. Planning is an async LLM
//! call; running it off the UI thread keeps the frame loop non-blocking. The
//! result is delivered over a `std::sync::mpsc` channel, drained each frame.
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use crew_hive::{Planner, TaskGraph};

/// Handle to an in-flight plan. `try_take` returns `None` until the planner
/// thread finishes, then `Some(Ok(graph))` or `Some(Err(message))` once.
#[allow(dead_code)]
pub struct PlanHandle {
    rx: Receiver<Result<TaskGraph, String>>,
}

impl PlanHandle {
    /// Non-blocking check for the planned graph.
    #[allow(dead_code)]
    pub fn try_take(&self) -> Option<Result<TaskGraph, String>> {
        self.rx.try_recv().ok()
    }
}

/// Spawn a worker thread that plans `goal` into a `TaskGraph` and sends the
/// result back. The thread owns a current-thread tokio runtime.
#[allow(dead_code)]
pub fn plan_goal(goal: String, planner: Arc<dyn Planner>) -> PlanHandle {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(format!("runtime: {e}")));
                return;
            }
        };
        let result =
            rt.block_on(async move { planner.plan(&goal).await.map_err(|e| e.to_string()) });
        let _ = tx.send(result);
    });
    PlanHandle { rx }
}
