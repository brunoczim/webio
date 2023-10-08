//! This module provides locks to be used within a single instance of a Rust
//! WebAssembly module. Why locks given that a WASM module is single-threaded?
//! Well, when using async Rust, a critical operation can be split by an
//! `.await` expression. In fact, these locks are designed specially for that:
//! ensuring a critical operation is performed as if it were atomic even if you
//! insert an `.await` between two steps.

mod mutex;
mod rw;

pub use mutex::{Mutex, MutexGuard};

pub use rw::{ReadGuard, RwLock, WriteGuard};
