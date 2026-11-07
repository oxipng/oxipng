//! Check if a reduction makes file smaller, and keep best reductions.
//! Works asynchronously when possible

use std::cell::RefCell;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::*},
};

use deflate::Deflater;
use indexmap::IndexSet;
use log::trace;
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::sync::mpsc::{Receiver, Sender, channel};

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
    pub nth: usize,
    pub description: String,
}

impl Candidate {
    pub fn cmp_key(&self) -> impl Ord + use<> {
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
    retain_idat: bool,
    nth: AtomicUsize,
    executed: Arc<AtomicUsize>,
    max_size: Arc<AtomicMin>,
    candidates: RefCell<Vec<Candidate>>,
    /// images are sent to the caller thread for evaluation
    #[cfg(feature = "parallel")]
    eval_channel: (Sender<Candidate>, Receiver<Candidate>),
}

impl Evaluator {
    pub fn new(
        deadline: Arc<Deadline>,
        filters: IndexSet<FilterStrategy>,
        deflater: Deflater,
        optimize_alpha: bool,
        threshold: f64,
        retain_idat: bool,
    ) -> Self {
        #[cfg(feature = "parallel")]
        let eval_channel = channel();
        Self {
            deadline,
            filters,
            deflater,
            optimize_alpha,
            retain_idat,
            nth: AtomicUsize::new(0),
            executed: Arc::new(AtomicUsize::new(0)),
            max_size: Arc::new(AtomicMin::new(None, threshold)),
            candidates: RefCell::new(Vec::new()),
            #[cfg(feature = "parallel")]
            eval_channel,
        }
    }

    /// Wait for all evaluations to finish and return the passing results
    pub fn get_candidates(self) -> Vec<Candidate> {
        let max_size = self.max_size.clone();
        let mut candidates = self.collect();
        if candidates.is_empty() {
            return candidates;
        }
        let max_size = max_size.get().unwrap();
        candidates.retain(|c| c.estimated_output_size <= max_size);
        candidates.sort_by_key(Candidate::cmp_key);
        candidates
    }

    #[cfg(feature = "parallel")]
    fn collect(self) -> Vec<Candidate> {
        let (eval_send, eval_recv) = self.eval_channel;
        // Disconnect the sender, breaking the loop in the thread
        drop(eval_send);
        let nth = self.nth.load(SeqCst);
        // Yield to ensure all evaluations are executed
        // This can prevent deadlocks when run within an existing rayon thread pool
        while self.executed.load(Relaxed) < nth {
            rayon::yield_local();
        }
        let mut candidates = self.candidates.into_inner();
        candidates.extend(eval_recv.iter());
        candidates
    }

    #[cfg(not(feature = "parallel"))]
    fn collect(self) -> Vec<Candidate> {
        self.candidates.into_inner()
    }

    /// Inject previously evaluated candidates to be reconsidered for the final selection
    pub fn inject_candidates(&self, candidates: Vec<Candidate>) {
        self.max_size.set_min(candidates[0].estimated_output_size);
        self.candidates.borrow_mut().extend(candidates);
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
        let retain_idat = self.retain_idat;
        let executed = self.executed.clone();
        let max_size = self.max_size.clone();
        let description = description.to_string();
        // sends it off asynchronously for compression,
        // but results will be collected via the message queue
        #[cfg(feature = "parallel")]
        let eval_send = self.eval_channel.0.clone();
        rayon::spawn(move || {
            executed.fetch_add(1, Relaxed);
            let filters_iter = filters.par_iter().with_max_len(1);
            filters_iter.for_each(|filter| {
                if deadline.passed() {
                    return;
                }
                let (filtered, filter_used) = image.filter_image(filter.clone(), optimize_alpha);
                let idat_data = deflater.deflate(&filtered, max_size.get());
                if let Ok(idat_data) = idat_data {
                    let size = image.estimated_output_size(&idat_data);
                    trace!(
                        "Eval: {}-bit {:23} {:8}   {} bytes",
                        image.ihdr.bit_depth, description, filter, size
                    );

                    // To save memory, only retain the IDAT when necessary
                    let new = Candidate {
                        image: image.clone(),
                        idat_data: if retain_idat { Some(idat_data) } else { None },
                        estimated_output_size: size,
                        filter: filter.clone(),
                        filter_used,
                        description: description.clone(),
                        nth,
                    };
                    max_size.set_min(size);

                    #[cfg(feature = "parallel")]
                    {
                        eval_send.send(new).expect("send");
                    }

                    #[cfg(not(feature = "parallel"))]
                    {
                        self.candidates.borrow_mut().push(new);
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

    /// Recompress candidates with the given deflater, returning the best result
    pub fn recompress_candidates(
        candidates: Vec<Candidate>,
        deadline: Arc<Deadline>,
        deflater: Deflater,
        optimize_alpha: bool,
        max_size: Option<usize>,
    ) -> Option<Candidate> {
        let max_size = AtomicMin::new(max_size, 0.0);
        let results_iter = candidates.into_par_iter().with_max_len(1);
        let results = results_iter.filter_map(|mut candidate| {
            if deadline.passed() {
                return None;
            }
            let (filtered, filter_used) = candidate
                .image
                .filter_image(candidate.filter_used, optimize_alpha);
            candidate.filter_used = filter_used;
            let idat_data = deflater.deflate(&filtered, max_size.get());
            if let Ok(idat_data) = idat_data {
                let size = candidate.image.estimated_output_size(&idat_data);
                trace!(
                    "Eval: {}-bit {:23} {:8}   {} bytes",
                    candidate.image.ihdr.bit_depth, candidate.description, candidate.filter, size
                );
                // Retain the IDAT on the candidate
                candidate.idat_data = Some(idat_data);
                candidate.estimated_output_size = size;
                max_size.set_min(size);
                return Some(candidate);
            } else if let Err(PngError::DeflatedDataTooLong(size)) = idat_data {
                trace!(
                    "Eval: {}-bit {:23} {:8}  >{} bytes",
                    candidate.image.ihdr.bit_depth, candidate.description, candidate.filter, size
                );
            }
            None
        });
        results.min_by_key(Candidate::cmp_key)
    }
}
