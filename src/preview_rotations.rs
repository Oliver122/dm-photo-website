//! In-memory preview rotation offsets (90° steps). Disk rewrite happens on confirm.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Per-job, per-relative-path quarter-turns (0..=3). Positive = CW from original.
#[derive(Clone, Default)]
pub struct PreviewRotationStore {
    inner: Arc<Mutex<HashMap<(i64, String), i8>>>,
}

impl PreviewRotationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply ±1 quarter-turn; returns new absolute quarters (0..=3).
    pub fn add_quarter(&self, job_id: i64, relative_path: &str, delta: i8) -> i8 {
        let mut map = self.inner.lock().expect("preview rotation lock");
        let key = (job_id, relative_path.to_string());
        let next = (map.get(&key).copied().unwrap_or(0) + delta).rem_euclid(4);
        map.insert(key, next);
        next
    }

    pub fn get_quarter(&self, job_id: i64, relative_path: &str) -> i8 {
        let map = self.inner.lock().expect("preview rotation lock");
        map.get(&(job_id, relative_path.to_string()))
            .copied()
            .unwrap_or(0)
    }

    pub fn snapshot_job(&self, job_id: i64) -> Vec<(String, i8)> {
        let map = self.inner.lock().expect("preview rotation lock");
        map.iter()
            .filter_map(|((jid, path), q)| {
                if *jid == job_id && *q != 0 {
                    Some((path.clone(), *q))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn clear_job(&self, job_id: i64) {
        let mut map = self.inner.lock().expect("preview rotation lock");
        map.retain(|(jid, _), _| *jid != job_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_after_four_cw() {
        let store = PreviewRotationStore::new();
        assert_eq!(store.add_quarter(1, "images/a.jpg", 1), 1);
        assert_eq!(store.add_quarter(1, "images/a.jpg", 1), 2);
        assert_eq!(store.add_quarter(1, "images/a.jpg", 1), 3);
        assert_eq!(store.add_quarter(1, "images/a.jpg", 1), 0);
    }

    #[test]
    fn ccw_from_zero_is_three() {
        let store = PreviewRotationStore::new();
        assert_eq!(store.add_quarter(1, "images/a.jpg", -1), 3);
    }
}
