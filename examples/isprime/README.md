# "'Is Prime' Test" Example

This is an example of usage of the crate `webio`, an asynchronous browser/node
runtime for Rust. It is a basic interface for testing whether a number is prime.

I know this is not the best example, because primality test is CPU-bound,
but I wanted to show an example of CPU-bound tasks running fast in WASM
without blocking the browser, i.e. by pausing periodically and giving
control back to browser by a few milliseconds.

A few numbers to try: `7399329281`, `2199023255551`, `9410454606139`,
`64954802446103`, `340845657750593`, `576460752303423487`,
`2305843009213693951`.

Note that the only JavaScript in this example is:
```javascript
import * as wasm from "isprime-wasm";
import * as style from "./style.css";

wasm.main();
```

That's it (ignoring webpack configuration file, of course).
