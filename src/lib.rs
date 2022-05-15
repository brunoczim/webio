//! This is an experimental implementation of a web-based async runtime. Web as
//! in "browser".
//!
//! Ideally, with this crate, the only JavaScript code you'll need to write
//! would be something like this:
//!
//! ```javascript
//! import * as wasm from "my-wasm-crate";
//! import * as style from "./style.css";
//!
//! wasm.main();
//! ```
//!
//! That's it (ignoring webpack configuration file, of course).
//!
//! # Examples
//!
//! ## Asynchronous "Is Prime" Test
//!
//! I know this is not the best example, because primality test is CPU-bound,
//! but I wanted to show an example of CPU-bound tasks running fast in WASM
//! without blocking the browser, i.e. by pausing periodically and giving
//! control back to browser by a few milliseconds.
//!
//! The full example can be seen in `examples/isprime` directory.
//!
//! A few numbers to try: `7399329281`, `2199023255551`, `9410454606139`,
//! `64954802446103`, `340845657750593`, `576460752303423487`,
//! `2305843009213693951`.
//!
//! ```no_run
//! use std::time::Duration;
//! use num::{BigUint, Zero};
//! use wasm_bindgen::JsValue;
//! use webio::event::{self, Type};
//!
//! /// Number of steps before yielding control back to browser when testing
//! /// whether a number is prime or not, in order not to freeze the browser with
//! /// computations on large numbers. Of course, yielding back to the browser is
//! /// just a pause, so after a few milliseconds later, WASM can resume its job on
//! /// the current number.
//! ///
//! /// However, note that this applies only when a number is being tested,
//! /// otherwise WASM sleeps and won't wake up until the button is pressed.
//! const YIELD_STEPS: u16 = 20000;
//!
//! /// Tests if the given number is prime, asynchronous because it will pause the
//! /// execution after some steps.
//! async fn is_prime(number: &BigUint) -> bool {
//!     let two = BigUint::from(2u8);
//!     if *number < two {
//!         return false;
//!     }
//!     if *number == two {
//!         return true;
//!     }
//!     if (number % &two).is_zero() {
//!         return false;
//!     }
//!     let mut attempt = BigUint::from(3u8);
//!     let mut square = &attempt * &attempt;
//!
//!     while square <= *number {
//!         if (number % &attempt).is_zero() {
//!             return false;
//!         }
//!         if (&attempt / &two % YIELD_STEPS).is_zero() {
//!             webio::time::timeout(Duration::from_millis(10)).await;
//!         }
//!         attempt += &two;
//!         square = &attempt * &attempt;
//!     }
//!
//!     true
//! }
//!
//! /// Main function of this WASM application.
//! #[webio::main]
//! pub async fn app_main() {
//!     // Gets all necessary HTML elements.
//!     let document = web_sys::window().unwrap().document().unwrap();
//!     let input_raw = document.get_element_by_id("input").unwrap();
//!     let input = web_sys::HtmlInputElement::from(JsValue::from(input_raw));
//!     let button = document.get_element_by_id("button").unwrap();
//!     let answer_elem = document.get_element_by_id("answer").unwrap();
//!
//!     // Sets a listener for the click event on the button.
//!     let listener = event::Click.add_listener(&button);
//!
//!     loop {
//!         // No problem being an infinite loop because it is asynchronous.
//!         // It won't block the browser.
//!         //
//!         // This will sleep until the user press a button.
//!         listener.listen_next().await.unwrap();
//!
//!         // Cleans up previous message.
//!         answer_elem.set_text_content(Some("Loading..."));
//!         // Gets and validates input.
//!         let number: BigUint = match input.value().parse() {
//!             Ok(number) => number,
//!             Err(_) => {
//!                 answer_elem.set_text_content(Some("Invalid input!"));
//!                 continue;
//!             },
//!         };
//!
//!         // Runs and tells the user the correct answer.
//!         if is_prime(&number).await {
//!             answer_elem.set_text_content(Some("Yes"));
//!         } else {
//!             answer_elem.set_text_content(Some("No"));
//!         }
//!     }
//! }
//! ```

#![warn(missing_docs)]
#![cfg_attr(feature = "feature-doc-cfg", feature(doc_cfg))]

#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
extern crate self as webio;

#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use webio_macros::{console, join, main, select, test, try_join};

#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use js_sys;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use wasm_bindgen;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use wasm_bindgen_futures;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use wasm_bindgen_test;
#[doc(hidden)]
#[cfg(feature = "macros")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "macros")))]
pub use web_sys;

#[macro_use]
mod macros;

pub mod task;

pub mod callback;

#[cfg(feature = "time")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "time")))]
pub mod time;

#[cfg(feature = "event")]
#[cfg_attr(feature = "feature-doc-cfg", doc(cfg(feature = "event")))]
pub mod event;
