//! The settings a broker child reads from its environment.
//!
//! WHY one place: the pane's broker (`chatspawn`) and the daemon's
//! (`daemon::procspawn`) each spawned their own child, and only the pane's
//! passed the spend cap, so a daemon pane ignored the user's setting, even 0
//! (off). Both now take the pairs from here.
use crate::config::CrewConfig;

impl CrewConfig {
    /// `NAME=value` pairs for a broker's environment.
    pub(crate) fn broker_env(&self) -> Vec<(String, String)> {
        vec![(
            "CREW_TASK_BUDGET_USD".into(),
            self.task_budget_usd.to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use crate::config::CrewConfig;

    #[test]
    fn the_spend_cap_reaches_a_broker_even_when_it_is_off() {
        let at = |usd: f64| {
            let c = CrewConfig {
                task_budget_usd: usd,
                ..CrewConfig::default()
            };
            c.broker_env()
        };
        assert_eq!(at(2.5), vec![("CREW_TASK_BUDGET_USD".into(), "2.5".into())]);
        assert_eq!(at(0.0), vec![("CREW_TASK_BUDGET_USD".into(), "0".into())]);
    }
}
