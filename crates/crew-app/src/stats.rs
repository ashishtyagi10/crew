use std::time::{Duration, Instant};

use sysinfo::{Disks, System};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub cpu: f32,
    pub mem: f32,
    pub disk: f32,
}

pub fn fraction(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        ((used as f64 / total as f64) as f32).clamp(0.0, 1.0)
    }
}

pub struct SysSampler {
    sys: System,
    disks: Disks,
    last: Option<Instant>,
    stats: Stats,
}

impl SysSampler {
    pub fn new() -> Self {
        let mut sampler = Self {
            sys: System::new(),
            disks: Disks::new_with_refreshed_list(),
            last: None,
            stats: Stats::default(),
        };
        sampler.sample();
        sampler.last = Some(Instant::now());
        sampler
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    pub fn refresh(&mut self) -> bool {
        let due = self
            .last
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(1000));
        if due {
            self.sample();
            self.last = Some(Instant::now());
            true
        } else {
            false
        }
    }

    fn sample(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.disks.refresh(false);

        let cpu = self.sys.global_cpu_usage().clamp(0.0, 100.0) / 100.0;
        let mem = fraction(self.sys.used_memory(), self.sys.total_memory());

        let (disk_used, disk_total) = self.disks.list().iter().fold((0u64, 0u64), |acc, d| {
            let used = d.total_space().saturating_sub(d.available_space());
            (acc.0 + used, acc.1 + d.total_space())
        });
        let disk = fraction(disk_used, disk_total);

        self.stats = Stats { cpu, mem, disk };
    }
}

impl Default for SysSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_zero_total() {
        assert_eq!(fraction(0, 0), 0.0);
    }

    #[test]
    fn fraction_half() {
        assert_eq!(fraction(50, 100), 0.5);
    }

    #[test]
    fn fraction_clamps_over_total() {
        assert_eq!(fraction(200, 100), 1.0);
    }

    #[test]
    fn stats_default() {
        assert_eq!(
            Stats::default(),
            Stats {
                cpu: 0.0,
                mem: 0.0,
                disk: 0.0
            }
        );
    }

    #[test]
    fn sampler_new_ranges() {
        let s = SysSampler::new().stats();
        assert!((0.0..=1.0).contains(&s.cpu));
        assert!((0.0..=1.0).contains(&s.mem));
        assert!((0.0..=1.0).contains(&s.disk));
    }
}
