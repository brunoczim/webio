initSidebarItems({"attr":[["main","This macro converts an asynchronous main function into a synchronous one that can actually be an entry point, and that invokes the asynchronous code. Under the hood, the asynchronous code is detached from the current call."],["test","This macro converts an asynchronous test function into a synchronous one that can actually be tested by `wasm_bindgen_test`, and that invokes the asynchronous code. Under the hood, the asynchronous code is detached from the current call."]],"derive":[["EventType","Defines a custom event wrapper, with the intention of being safe. It is up to the caller type, however, to ensure that name is correct for the given event data type."]],"macro":[["console","Prints to the JavaScript/browser/node console using a given method."],["console_debug","Debugs to the JavaScript/browser/node console using a given method. Syntax:"],["console_error","Shows error in the JavaScript/browser/node console using a given method."],["console_info","Shows info in the JavaScript/browser/node console using a given method."],["console_log","Logs to the JavaScript/browser/node console using a given method. Syntax:"],["console_warn","Warns to the JavaScript/browser/node console using a given method. Syntax:"],["join","Joins a list of futures and returns their output into a tuple in the same order that the futures were given. Futures must be `'static`."],["run_tests_in_browser","Flags a test file as running in the browser instead of node."],["select","Listens to a list of futures and finishes when the first future finishes, which is then selected. Every future is placed in a “match arm”, and when it is selected, the “arm” pattern is matched and the macro evaluates to the right side of the “arm”. Patterns must be irrefutable, typically just a variable name, or destructuring. Futures must be `'static`."],["try_join","Joins a list of futures and returns their output into a tuple in the same order that the futures were given, but if one of them fails, `try_join` fails, and so a result of tuples is returned. Futures must be `'static`."]],"mod":[["callback","This module defines utilities for translating a callback into asynchronous functions."],["event","Module for listening and handling JS events from Rust."],["task","This module exports items related to task spawning."],["time","This module implements time-related utilities."]]});