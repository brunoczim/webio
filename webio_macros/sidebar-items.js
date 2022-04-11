initSidebarItems({"attr":[["main","This macro converts an asynchronous main function into a synchronous one that can actually be an entry point, and that invokes the asynchronous code. Under the hood, the asynchronous code is detached from the current call."],["test","This macro converts an asynchronous test function into a synchronous one that can actually be tested by `wasm_bindgen_test`, and that invokes the asynchronous code. Under the hood, the asynchronous code is detached from the current call."]],"macro":[["console","Prints to the JavaScript/browser/node console using a given method."],["join","Joins a list of futures and returns their output into a tuple in the same order that the futures were given. Futures must be `'static`."],["select","Listens to a list of futures and finishes when the first future finishes, which is then selected. Every future is placed in a “match arm”, and when it is selected, the “arm” pattern is matched and the macro evaluates to the right side of the “arm”. Patterns must be irrefutable, typically just a variable name, or destructuring. Futures must be `'static`."],["try_join","Joins a list of futures and returns their output into a tuple in the same order that the futures were given, but if one of them fails, `try_join` fails, and so a result of tuples is returned. Futures must be `'static`."]]});