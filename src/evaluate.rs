//! Check if a reduction makes file smaller, and keep best reductions.
//! Works asynchronously when possible

#[cfg(not(feature = "parallel"))]
use std::cell::RefCell;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::*},
};

#[cfg(feature = "parallel")]
use crossbeam_channel::{Receiver, Sender, unbounded};
use deflate::Deflater;
use indexmap::IndexSet;
use log::trace;
use rayon::prelude::*;

#[cfg(not(feature = "parallel"))]
use crate::rayon;
use crate::{
    Deadline, PngError, atomicmin::AtomicMin, deflate, filters::FilterStrategy, png::PngImage,
};

pub(crate) struct Candidate {
    pub image: Arc<PngImage>,
    pub idat_data: Option<Vec<u8>>,
    pub estimated_output_size: usize,
    /// The input filter, which is retained for printing and for APNG frames.
    pub filter: FilterStrategy,
    /// The filter returned by the filter function, which may be Predefined.
    /// Use this for the next round to avoid recomputing the filter.
    pub filter_used: FilterStrategy,
    /// For determining tie-breaker
    nth: usize,
}

impl Candidate {
    fn cmp_key(&self) -> impl Ord + use<> {
        (
            self.estimated_output_size,
            self.image.data.len(),
            self.filter.clone(),
            // Prefer the later image added (e.g. baseline, which is always added last)
            usize::MAX - self.nth,
        )
    }
}

/// Collect image versions and pick one that compresses best
pub(crate) struct Evaluator {
    deadline: Arc<Deadline>,
    filters: IndexSet<FilterStrategy>,
    deflater: Deflater,
    optimize_alpha: bool,
    final_round: bool,
    nth: AtomicUsize,
    executed: Arc<AtomicUsize>,
    best_candidate_size: Arc<AtomicMin>,
    /// images are sent to the caller thread for evaluation
    #[cfg(feature = "parallel")]
    eval_channel: (Sender<Candidate>, Receiver<Candidate>),
    // in non-parallel mode, images are evaluated synchronously
    #[cfg(not(feature = "parallel"))]
    eval_best_candidate: RefCell<Option<Candidate>>,
}

impl Evaluator {
    pub fn new(
        deadline: Arc<Deadline>,
        filters: IndexSet<FilterStrategy>,
        deflater: Deflater,
        optimize_alpha: bool,
        final_round: bool,
    ) -> Self {
        #[cfg(feature = "parallel")]
        let eval_channel = unbounded();
        Self {
            deadline,
            filters,
            deflater,
            optimize_alpha,
            final_round,
            nth: AtomicUsize::new(0),
            executed: Arc::new(AtomicUsize::new(0)),
            best_candidate_size: Arc::new(AtomicMin::new(None)),
            #[cfg(feature = "parallel")]
            eval_channel,
            #[cfg(not(feature = "parallel"))]
            eval_best_candidate: RefCell::new(None),
        }
    }

    /// Wait for all evaluations to finish and return smallest reduction
    /// Or `None` if the queue is empty.
    #[cfg(feature = "parallel")]
    pub fn get_best_candidate(self) -> Option<Candidate> {
        let (eval_send, eval_recv) = self.eval_channel;
        // Disconnect the sender, breaking the loop in the thread
        drop(eval_send);
        let nth = self.nth.load(SeqCst);
        // Yield to ensure all evaluations are executed
        // This can prevent deadlocks when run within an existing rayon thread pool
        while self.executed.load(Relaxed) < nth {
            rayon::yield_local();
        }
        eval_recv.into_iter().min_by_key(Candidate::cmp_key)
    }

    #[cfg(not(feature = "parallel"))]
    pub fn get_best_candidate(self) -> Option<Candidate> {
        self.eval_best_candidate.into_inner()
    }

    /// Set best size, if known in advance
    pub fn set_best_size(&self, size: usize) {
        self.best_candidate_size.set_min(size);
    }

    /// Check if the image is smaller than others
    pub fn try_image(&self, image: Arc<PngImage>) {
        let description = image.ihdr.color_type.to_string();
        self.try_image_with_description(image, &description);
    }

    /// Check if the image is smaller than others, with a description for verbose mode
    pub fn try_image_with_description(&self, image: Arc<PngImage>, description: &str) {
        let nth = self.nth.fetch_add(1, SeqCst);
        // These clones are only cheap refcounts
        let deadline = self.deadline.clone();
        let filters = self.filters.clone();
        let deflater = self.deflater;
        let optimize_alpha = self.optimize_alpha;
        let final_round = self.final_round;
        let executed = self.executed.clone();
        let best_candidate_size = self.best_candidate_size.clone();
        let description = description.to_string();
        // sends it off asynchronously for compression,
        // but results will be collected via the message queue
        #[cfg(feature = "parallel")]
        let eval_send = self.eval_channel.0.clone();
        rayon::spawn(move || {
            executed.fetch_add(1, Relaxed);
            let filters_iter = filters.par_iter().with_max_len(1);

            // Updating of best result inside the parallel loop would require locks,
            // which are dangerous to do in side Rayon's loop.
            // Instead, only update (atomic) best size in real time,
            // and the best result later without need for locks.
            filters_iter.for_each(|filter| {
                if deadline.passed() {
                    return;
                }
                let (filtered, filter_used) = image.filter_image(filter.clone(), optimize_alpha);
                let idat_data = deflater.deflate(&filtered, best_candidate_size.get());
                if let Ok(idat_data) = idat_data {
                    let estimated_output_size = image.estimated_output_size(&idat_data);
                    trace!(
                        "Eval: {}-bit {:23} {:8}   {} bytes",
                        image.ihdr.bit_depth, description, filter, estimated_output_size
                    );

                    // Skip if it exceeds best known size. (This is important to ensure
                    // the evaluator returns no result when all candidates are too large.)
                    if let Some(max) = best_candidate_size.get() {
                        if estimated_output_size > max {
                            return;
                        }
                    }

                    // We only need to retain the IDAT data in the final round
                    let new = Candidate {
                        image: image.clone(),
                        idat_data: if final_round { Some(idat_data) } else { None },
                        estimated_output_size,
                        filter: filter.clone(),
                        filter_used,
                        nth,
                    };
                    best_candidate_size.set_min(estimated_output_size);

                    #[cfg(feature = "parallel")]
                    {
                        eval_send.send(new).expect("send");
                    }

                    #[cfg(not(feature = "parallel"))]
                    {
                        match &mut *self.eval_best_candidate.borrow_mut() {
                            Some(prev) if prev.cmp_key() < new.cmp_key() => {}
                            best => *best = Some(new),
                        }
                    }
                } else if let Err(PngError::DeflatedDataTooLong(size)) = idat_data {
                    trace!(
                        "Eval: {}-bit {:23} {:8}  >{} bytes",
                        image.ihdr.bit_depth, description, filter, size
                    );
                }
            });
        });
    }
}
