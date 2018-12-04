//! Thread safe generator of continuous ids

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};

pub type ID = usize;
pub type Generator = AtomicUsize;
pub const GENERATOR_INIT: AtomicUsize = ATOMIC_USIZE_INIT;

macro_rules! next_id {
    ($generator: ident) => ({
        use std::sync::atomic::Ordering;
        $generator.fetch_add(1, Ordering::SeqCst)
    })
}
