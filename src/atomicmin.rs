use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

#[derive(Debug)]
pub struct AtomicMin {
    val: AtomicUsize,
    plus_factor: f64,
}

impl AtomicMin {
    #[must_use]
    pub fn new(init: Option<usize>, plus_factor: f64) -> Self {
        Self {
            val: AtomicUsize::new(init.unwrap_or(usize::MAX)),
            plus_factor,
        }
    }

    pub fn get(&self) -> Option<usize> {
        let val = self.val.load(SeqCst);
        if val == usize::MAX { None } else { Some(val) }
    }

    /// Try a new value, returning true if it is the new minimum
    pub fn set_min(&self, new_val: usize) -> bool {
        let adjusted_val = new_val + (self.plus_factor * new_val as f64) as usize;
        new_val < self.val.fetch_min(adjusted_val, SeqCst)
    }
}
