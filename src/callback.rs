//! This module defines utilities for translating a callback into asynchronous
//! functions.

mod shared;

pub mod multi;
pub mod once;

pub use shared::Error;
