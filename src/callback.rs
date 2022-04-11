//! This module defines utilities for translating a callback into asynchronous
//! functions.

mod shared;

pub mod once;
pub mod multi;

pub use shared::Error;
