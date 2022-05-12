//! Implementation mimicking [`std::time::Instant`].

use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast};

// I cannot simply declare `fn performance_now()` with
// `(j́s_name = "now", namespace = "performance")` because wasm-pack generates
// code that webpack does not handle correctly, probably because of hoisting.
// And so, I need this workaround.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = ::js_sys::Object, js_name = Object)]
    type Global;

    #[wasm_bindgen(method, structural, getter)]
    fn performance(this: &Global) -> Performance;

    #[wasm_bindgen(extends = ::js_sys::Object, js_name = Performance)]
    type Performance;

    #[wasm_bindgen(method, structural)]
    fn now(this: &Performance) -> f64;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "now", js_namespace = performance)]
    fn performance_now() -> f64;
}

#[cold]
#[inline(never)]
fn earlier_is_actually_later() -> ! {
    panic!("The 'earlier' instant is actually later")
}

#[cold]
#[inline(never)]
fn instant_is_infinite() -> ! {
    panic!("Instant is infinite")
}

/// A montonic clock measurement, mimicking [`std::time::Instant`] but for WASM,
/// which is not supported by std's `Instant`. Behind the curtains, this type
/// uses JavaScript's `performance::now()`, so there is an overhead when
/// calling [`Instant::now`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Instant {
    millis: f64,
}

impl Instant {
    /// Gets the clock measurement for this right moment. This uses JS
    /// `performance_now`, so there might an overhead calling this method.
    pub fn now() -> Self {
        let global = js_sys::global().dyn_into::<Global>().unwrap();
        let millis = global.performance().now();
        Self { millis }
    }

    /// Returns the duration of time that passed from an earlier instant into
    /// this instant. If the `earlier` instant actually happened after the
    /// current instant, `None` is returned.
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        let millis = self.millis - earlier.millis;
        if millis >= 0.0 {
            Some(Duration::from_secs_f64(millis / 1000.0))
        } else {
            None
        }
    }

    /// Returns the duration of time that passed from an earlier instant into
    /// this instant. If the `earlier` instant actually happened after the
    /// current instant, a zeroed duration is returned.
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.checked_duration_since(earlier).unwrap_or(Duration::from_secs(0))
    }

    /// Returns the duration of time that passed from an earlier instant into
    /// this instant.
    ///
    /// # Panics
    ///
    /// Panics if the `earlier` instant actually happened after the current
    /// instant.
    pub fn duration_since(self, earlier: Self) -> Duration {
        match self.checked_duration_since(earlier) {
            Some(duration) => duration,
            None => earlier_is_actually_later(),
        }
    }

    /// Returns the duration of time that has passed since this instant was
    /// created.
    pub fn elapsed(self) -> Duration {
        Self::now().duration_since(self)
    }

    /// Adds a duration of time to this instant. If the resulting instant
    /// becomes too big towards infinite, `None` is returned.
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        let millis = self.millis + (duration.as_secs_f64() * 1000.0);
        if millis.is_finite() {
            Some(Self { millis })
        } else {
            None
        }
    }

    /// Subtracts a duration of time from this instant. If the resulting instant
    /// becomes too small towards negative infinite, `None` is returned.
    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        let millis = self.millis - (duration.as_secs_f64() * 1000.0);
        if millis.is_finite() {
            Some(Self { millis })
        } else {
            None
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, duration: Duration) -> Self::Output {
        match self.checked_add(duration) {
            Some(output) => output,
            None => instant_is_infinite(),
        }
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self::Output {
        match self.checked_sub(duration) {
            Some(output) => output,
            None => instant_is_infinite(),
        }
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}

impl Eq for Instant {}

impl Ord for Instant {
    fn cmp(&self, other: &Self) -> Ordering {
        self.millis.partial_cmp(&other.millis).unwrap()
    }
}

impl Hash for Instant {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.millis.to_bits().hash(state)
    }
}
