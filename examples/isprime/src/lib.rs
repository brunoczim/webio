mod utils;

use num::{BigUint, Zero};
use std::time::Duration;
use wasm_bindgen::JsValue;
use webio::{
    event::{self, EventType},
    time::Instant,
};

/// Number of steps before yielding control back to browser when testing
/// whether a number is prime or not, in order not to freeze the browser with
/// computations on large numbers. Of course, yielding back to the browser is
/// just a pause, so after a few milliseconds later, WASM can resume its job on
/// the current number.
///
/// However, note that this applies only when a number is being tested,
/// otherwise WASM sleeps and won't wake up until the button is pressed.
const YIELD_STEPS: u16 = 20000;

/// Tests if the given number is prime, asynchronous because it will pause the
/// execution after some steps.
async fn is_prime(number: &BigUint) -> bool {
    let two = BigUint::from(2u8);
    if *number < two {
        return false;
    }
    if *number == two {
        return true;
    }
    if (number % &two).is_zero() {
        return false;
    }
    let mut attempt = BigUint::from(3u8);
    let mut square = &attempt * &attempt;

    while square <= *number {
        if (number % &attempt).is_zero() {
            return false;
        }
        if (&attempt / &two % YIELD_STEPS).is_zero() {
            webio::time::timeout(Duration::from_millis(10)).await;
        }
        attempt += &two;
        square = &attempt * &attempt;
    }

    true
}

/// Main function of this WASM application.
#[webio::main]
pub async fn main() {
    // Sets a panic hook that allows us to see panic in console.
    utils::set_panic_hook();

    // Gets all necessary HTML elements.
    let document = web_sys::window().unwrap().document().unwrap();
    let input_raw = document.get_element_by_id("input").unwrap();
    let input = web_sys::HtmlInputElement::from(JsValue::from(input_raw));
    let button = document.get_element_by_id("button").unwrap();
    let answer_elem = document.get_element_by_id("answer").unwrap();
    let time_elem = document.get_element_by_id("time").unwrap();

    // Sets a listener for the click event on the button.
    let listener = event::Click.add_listener(&button);

    loop {
        // No problem being an infinite loop because it is asynchronous.
        // It won't block the browser.
        //
        // This will sleep until the user press a button.
        listener.listen_next().await.unwrap();

        // Cleans up previous message.
        answer_elem.set_text_content(Some("Loading..."));
        time_elem.set_text_content(Some("?"));
        // Gets and validates input.
        let number: BigUint = match input.value().parse() {
            Ok(number) => number,
            Err(_) => {
                answer_elem.set_text_content(Some("Invalid input!"));
                continue;
            },
        };

        // Gets time before running.
        let then = Instant::now();
        // Runs.
        let answer = is_prime(&number).await;
        // Gets elapsed time since before running.
        let elapsed = then.elapsed();
        // Tells the user the correct answer.
        if answer {
            answer_elem.set_text_content(Some("Yes"));
        } else {
            answer_elem.set_text_content(Some("No"));
        }
        // Shows the user the time spent.
        let formatted_time =
            format!("{} ms", elapsed.as_secs_f64() * 1000.0);
        time_elem.set_text_content(Some(&formatted_time));
    }
}
