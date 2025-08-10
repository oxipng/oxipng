use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

#[derive(Debug)]
pub struct AtomicMin {
    val: AtomicUsize,
}

impl AtomicMin {
    #[must_use]
    pub fn new(init: Option<usize>) -> Self {
        Self {
            val: AtomicUsize::new(init.unwrap_or(usize::MAX)),
        }
    }

    pub fn get(&self) -> Option<usize> {
        let val = self.val.load(SeqCst);
        if val == usize::MAX { None } else { Some(val) }
    }

    /// Try a new value, returning true if it is the new minimum
    pub fn set_min(&self, new_val: usize) -> bool {
        new_val < self.val.fetch_min(new_val, SeqCst)
    }
}
