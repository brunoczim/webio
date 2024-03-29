use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, ItemFn, Visibility};

mod error;
mod join;
mod select;
mod console;
mod event_type;

/// Joins a list of futures and returns their output into a tuple in the same
/// order that the futures were given. Futures must be `'static`.
///
/// Syntax:
///
/// ```ignore
/// join!(future0, future1, future2, ..., future_n)
/// ```
///
/// # Examples
///
/// ## With Timeout
/// ```ignore
/// use std::time::Duration;
/// use webio::{join, time::timeout};
///  
/// # fn main() {
/// # task::detach(async {
///
/// // Create some tasks
/// let first_handle = async {
///     timeout(Duration::from_millis(50)).await;
///     3
/// };
/// let second_handle = async {
///     timeout(Duration::from_millis(60)).await;
///     5
/// };
/// let third_handle = async {
///     timeout(Duration::from_millis(40)).await;
///     7
/// };
///
/// // Join them
/// let (first, second, third) = join!(first_handle, second_handle, third_handle);
///
/// // Expected output
/// assert_eq!((first, second, third), (3, 5, 7));
/// # });
/// # }
/// ```
#[proc_macro]
pub fn join(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as join::Input);
    let futures = input.futures;

    let future_var_names = || {
        (0 .. futures.len())
            .map(|i| Ident::new(&format!("future{}", i), Span::mixed_site()))
    };

    let future_decls =
        future_var_names().zip(&futures).map(|(ident, future)| {
            quote! { let #ident = #future; }
        });

    let output_var_names = || {
        (0 .. futures.len())
            .map(|i| Ident::new(&format!("output{}", i), Span::mixed_site()))
    };

    let output_decls = output_var_names().map(|ident| {
        quote! {
            let #ident = ::std::rc::Rc::new(::std::cell::Cell::new(None));
        }
    });

    let adaptor_var_names = || {
        (0 .. futures.len())
            .map(|i| Ident::new(&format!("adaptor{}", i), Span::mixed_site()))
    };

    let adaptor_decls = adaptor_var_names()
        .zip(future_var_names())
        .zip(output_var_names())
        .map(|((adaptor, future), output)| {
            quote! {
                let #adaptor = {
                    let #output = #output.clone();
                    async move {
                        let output_val = #future.await;
                        #output.set(Some(output_val));
                        Ok(::webio::wasm_bindgen::JsValue::UNDEFINED)
                    }
                };
            }
        });

    let promise_var_names = || {
        (0 .. futures.len())
            .map(|i| Ident::new(&format!("promise{}", i), Span::mixed_site()))
    };

    let promise_decls = promise_var_names().zip(adaptor_var_names()).map(
        |(promise, adaptor)| {
            quote! {
                let #promise = ::webio::wasm_bindgen::JsValue::from(
                    ::webio::wasm_bindgen_futures::future_to_promise(#adaptor)
                );
            }
        },
    );

    let promise_var_names_iter = promise_var_names();
    let output_iter =
        output_var_names().map(|ident| quote! { #ident.take().unwrap() });

    let expanded = quote! {
        {
            #(#future_decls)*
            #(#output_decls)*
            #(#adaptor_decls)*
            #(#promise_decls)*
            let mut promise_list = ::webio::js_sys::Array::new();
            promise_list.extend([#(#promise_var_names_iter),*]);
            let final_promise = ::webio::js_sys::Promise::all(&promise_list);
            ::webio::wasm_bindgen_futures::JsFuture::from(final_promise)
                .await
                .unwrap();
            (#(#output_iter),*)
        }
    };
    expanded.into()
}

/// Joins a list of futures and returns their output into a tuple in the same
/// order that the futures were given, but if one of them fails, `try_join`
/// fails, and so a result of tuples is returned. Futures must be `'static`.
///
/// Syntax:
///
/// ```ignore
/// try_join!(future0, future1, future2, ..., future_n)
/// ```
///
/// # Examples
///
/// ## With Timeout and Success
/// ```ignore
/// use std::time::Duration;
/// use webio::{try_join, time::timeout};
///
/// # fn main() {
/// # task::detach(async {
/// // Create some tasks
/// let first_handle = async {
///     timeout(Duration::from_millis(50)).await;
///     Result::<u32, &str>::Ok(3)
/// };
/// let second_handle = async {
///     timeout(Duration::from_millis(60)).await;
///     Ok(5)
/// };
/// let third_handle = async {
///     timeout(Duration::from_millis(40)).await;
///     Ok(7)
/// };
///
/// // Try to join it
/// let result = try_join!(first_handle, second_handle, third_handle);
/// // Should be Ok
/// let (first, second, third) = result.unwrap();
///
/// // Expected output
/// assert_eq!((first, second, third), (3, 5, 7));
/// # });
/// # }
/// ```
///
/// ## With Timeout and Failure
/// ```ignore
/// use std::time::Duration;
/// use webio::{try_join, time::timeout};
///
/// # fn main() {
/// # task::detach(async {
/// // Create some tasks
/// let first_handle = async {
///     timeout(Duration::from_millis(50)).await;
///     Ok(3)
/// };
/// let second_handle = async {
///     timeout(Duration::from_millis(60)).await;
///     Err("boom")
/// };
/// let third_handle = async {
///     timeout(Duration::from_millis(40)).await;
///     Ok(7)
/// };
///
/// // Try to join them
/// let result = try_join!(first_handle, second_handle, third_handle);
///
/// // Should be an error
/// assert_eq!(result, Err("boom"));
/// # });
/// # }
/// ```
#[proc_macro]
pub fn try_join(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as join::Input);
    let futures = input.futures;
    let output_var_names = || {
        (0 .. futures.len())
            .map(|i| Ident::new(&format!("output{}", i), Span::mixed_site()))
    };
    let output_iter = output_var_names();
    let output_try_iter = output_var_names();
    let expanded = quote! {
        async move {
            let (#(#output_iter),*) = ::webio::join!(#(#futures),*);
            Ok((#(match #output_try_iter {
                Ok(output) => output,
                Err(error) => return Err(error),
            }),*))
        }.await
    };
    expanded.into()
}

/// Listens to a list of futures and finishes when the first future finishes,
/// which is then selected. Every future is placed in a "match arm", and when it
/// is selected, the "arm" pattern is matched and the macro evaluates to the
/// right side of the "arm". Patterns must be irrefutable, typically just a
/// variable name, or destructuring. Futures must be `'static`.
///
/// Syntax:
///
/// ```ignore
/// select! {
///     pattern0 = future0 => output0,
///     pattern1 = future1 => output1,
///     pattern2 = future2 => output2,
///     ...,
///     pattern_n = future_n => output_n,
/// }
/// ```
///
/// # Examples
///
/// ## With Timeout
///
/// ```ignore
/// use std::time::Duration;
/// use webio::{select, time::timeout};
///
/// # fn main () {
/// # task::detach(async {
/// // Create some tasks
/// let first_handle = async {
///     timeout(Duration::from_millis(500)).await;
///     3u32
/// };
/// let second_handle = async {
///     timeout(Duration::from_millis(50)).await;
///     5u32
/// };
/// let third_handle = async {
///     timeout(Duration::from_millis(350)).await;
///     7u32
/// };
///
/// // Select the first one to complete
/// let output = select! {
///     val = first_handle => val + 10,
///     val = second_handle => val + 20,
///     val = third_handle => val - 5
/// };
///
/// // Second one should be the first
/// assert_eq!(output, 25);
/// # });
/// # }
/// ```
#[proc_macro]
pub fn select(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as select::Input);
    let arms = input.arms;

    let future_var_names = || {
        (0 .. arms.len())
            .map(|i| Ident::new(&format!("future{}", i), Span::mixed_site()))
    };

    let future_decls = future_var_names()
        .zip(arms.iter().map(|arm| &arm.future))
        .map(|(ident, future)| {
            quote! { let #ident = #future; }
        });

    let output_var_name = Ident::new("output", Span::mixed_site());

    let output_decl = quote! {
        let #output_var_name= ::std::rc::Rc::new(::std::cell::Cell::new(None));
    };

    let adaptor_var_names = || {
        (0 .. arms.len())
            .map(|i| Ident::new(&format!("adaptor{}", i), Span::mixed_site()))
    };

    let adaptor_decls = adaptor_var_names()
        .zip(future_var_names())
        .zip(&arms)
        .map(|((adaptor, future), arm)| {
            let pat = &arm.pattern;
            let final_output = &arm.output;
            quote! {
                let #adaptor = {
                    let #output_var_name = #output_var_name.clone();
                    async move {
                        let output_val = #future.await;
                        let mut stored_output = #output_var_name.take();
                        if stored_output.is_none() {
                            let #pat = output_val;
                            stored_output = Some(#final_output);
                        }
                        #output_var_name.set(stored_output);
                        Ok(::webio::wasm_bindgen::JsValue::UNDEFINED)
                    }
                };
            }
        });

    let promise_var_names = || {
        (0 .. arms.len())
            .map(|i| Ident::new(&format!("promise{}", i), Span::mixed_site()))
    };

    let promise_decls = promise_var_names().zip(adaptor_var_names()).map(
        |(promise, adaptor)| {
            quote! {
                let #promise = ::webio::wasm_bindgen::JsValue::from(
                    ::webio::wasm_bindgen_futures::future_to_promise(#adaptor)
                );
            }
        },
    );

    let promise_var_names_iter = promise_var_names();

    let expanded = quote! {
        {
            #(#future_decls)*
            #output_decl
            #(#adaptor_decls)*
            #(#promise_decls)*
            let mut promise_list = ::webio::js_sys::Array::new();
            promise_list.extend([#(#promise_var_names_iter),*]);
            let final_promise = ::webio::js_sys::Promise::any(&promise_list);
            ::webio::wasm_bindgen_futures::JsFuture::from(final_promise)
                .await
                .unwrap();
            #output_var_name.take().unwrap()
        }
    };

    expanded.into_token_stream().into()
}

/// Prints to the JavaScript/browser/node console using a given method.
///
/// Syntax:
///
/// ```ignore
/// console!($method; argument0, argument1, argument2, ..., argument_n)
/// ```
///
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ## Log Different ARgument Types
///
/// ```ignore
/// use webio::console;
/// # fn main() {
/// console!(log; "Hello number", 5u8, "you're welcome!");
/// # }
/// ```
#[proc_macro]
pub fn console(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as console::Input);

    let expanded = if input.arguments.len() < 8 {
        let method = Ident::new(
            &format!("{}_{}", input.method, input.arguments.len()),
            Span::mixed_site(),
        );
        let arguments = input.arguments.into_iter().map(|argument| {
            quote! {
                &Into::<::webio::wasm_bindgen::JsValue>::into(#argument)
            }
        });
        quote! {
            ::webio::web_sys::console::#method(#(#arguments),*)
        }
    } else {
        let method = input.method;
        let arguments = input.arguments.into_iter().map(|argument| {
            quote! {
                array.push(
                    &Into::<::webio::wasm_bindgen::JsValue>::into(#argument)
                );
            }
        });
        quote! {
            {
                let mut array = ::webio::js_sys::Array::new();
                #(#arguments)*
                ::webio::web_sys::console::#method(&array)
            }
        }
    };

    expanded.into()
}

/// This macro converts an asynchronous main function into a synchronous one
/// that can actually be an entry point, and that invokes the asynchronous code.
/// Under the hood, the asynchronous code is detached from the current call.
///
/// # Examples
///
/// ## Main With Timeout
///
/// ```ignore
/// use webio::time::timeout;
/// use std::time::Duration;
///
/// #[webio::main]
/// async fn main() {
///     timeout(Duration::from_millis(200)).await;
/// }
/// ```
#[proc_macro_attribute]
pub fn main(raw_attribute: TokenStream, raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as ItemFn);
    let mut error_dump = error::Dump::new();

    if !raw_attribute.is_empty() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main attribute must not receive arguments",
        ));
    }
    if input.sig.asyncness.is_none() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main must be an asynchronous function with async syntax",
        ));
    }
    if input.sig.constness.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main cannot be const",
        ));
    }
    if input.sig.inputs.len() > 0 || input.sig.variadic.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main function cannot receive parameters",
        ));
    }
    if input.sig.abi.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main does not support ABIs",
        ));
    }
    if input.sig.unsafety.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main cannot be unsafe",
        ));
    }
    if !matches!(input.vis, Visibility::Public(_)) {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::main must be public",
        ));
    }

    match error_dump.into_errors() {
        Some(stored) => stored.into_compile_error().into(),
        None => {
            let fn_token = input.sig.fn_token;
            let ident = input.sig.ident;
            let body = input.block;
            let attrs = input.attrs;
            let expanded = quote! {
                #[::webio::wasm_bindgen::prelude::wasm_bindgen(start)]
                #(#attrs)*
                pub async #fn_token #ident() {
                    let (): () = #body;
                }
            };
            expanded.into_token_stream().into()
        },
    }
}

/// This macro converts an asynchronous test function into a synchronous one
/// that can actually be tested by `wasm_bindgen_test`, and that invokes the
/// asynchronous code. Under the hood, the asynchronous code is detached from
/// the current call.
///
/// # Examples
///
/// ## Test With Timeout
///
/// ```ignore
/// use webio::time::timeout;
/// use std::time::Duration;
///
/// #[webio::test]
/// async fn my_test() {
///     timeout(Duration::from_millis(200)).await;
///     assert!(true);
/// }
/// ```
#[proc_macro_attribute]
pub fn test(raw_attribute: TokenStream, raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as ItemFn);

    /*
    let should_panic_attr_pos = input.attrs.iter().position(|attr| {
        matches!(attr.style, syn::AttrStyle::Outer)
            && attr.path.segments.len() == 1
            && attr.path.segments[0].ident == "should_panic"
            && attr.path.segments[0].arguments.is_empty()
            && attr.tokens.is_empty()
    });
    let should_panic = match should_panic_attr_pos {
        Some(pos) => {
            input.attrs.remove(pos);
            true
        },
        None => false,
    };
    */

    let mut error_dump = error::Dump::new();

    if !raw_attribute.is_empty() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::test attribute must not receive arguments",
        ));
    }
    if input.sig.asyncness.is_none() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::test must be an asynchronous function with async syntax",
        ));
    }
    if input.sig.constness.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::test cannot be const",
        ));
    }
    if input.sig.inputs.len() > 0 || input.sig.variadic.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::test function cannot receive parameters",
        ));
    }
    if input.sig.abi.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::test does not support ABIs",
        ));
    }
    if input.sig.unsafety.is_some() {
        error_dump.append(syn::Error::new(
            Span::call_site(),
            "webio::test cannot be unsafe",
        ));
    }

    match error_dump.into_errors() {
        Some(stored) => stored.into_compile_error().into(),
        None => {
            let visibility = input.vis;
            let fn_token = input.sig.fn_token;
            let ident = input.sig.ident;
            let body = input.block;
            let attrs = input.attrs;
            let expanded = quote! {
                #[::webio::wasm_bindgen_test::wasm_bindgen_test]
                #(#attrs)*
                #visibility async #fn_token #ident() {
                    webio::set_test_panic_hook();
                    let (): () = #body;
                }
            };
            expanded.into_token_stream().into()
        },
    }
}

/// Defines a custom event wrapper, with the intention of being safe. It is up
/// to the caller type, however, to ensure that name is correct for the given
/// event data type.
///
/// It is required that `event_type(name = ..., data = ...)` attribute is
/// required, and should be placed at the top of the struct; it is ignored
/// elsewhere.
///
/// # Example
///
/// ```ignore
/// #[derive(EventType)]
/// #[event_type(name = "click", data = web_sys::MouseEvent)]
/// struct CustomClick;
///
/// # fn main() {
/// # let element = todo!();
/// let listener = CustomClick.add_listener(&element);
///
/// element.dispatch_event(&web_sys::MouseEvent::new("click").unwrap()).unwrap();
/// listener.listen_next().await.unwrap();
/// element.dispatch_event(&web_sys::MouseEvent::new("click").unwrap()).unwrap();
/// listener.listen_next().await.unwrap();
/// # }
/// ```
#[proc_macro_derive(EventType, attributes(event_type))]
pub fn event_type(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as DeriveInput);
    let data = match &input.data {
        Data::Struct(strut) => strut,
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "EventType supports only unit structs",
            )
            .into_compile_error()
            .into()
        },
    };
    match &data.fields {
        Fields::Unit => (),
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "EventType supports only unit structs",
            )
            .into_compile_error()
            .into()
        },
    }

    let typ = input.ident;

    let mut error_dump = error::Dump::new();
    let mut partial_args = event_type::PartialArguments::new();
    for attr in input.attrs {
        match attr.path().get_ident() {
            Some(ident) if ident == "event_type" => match attr.parse_args() {
                Ok(current_partial_args) => {
                    if let Err(error) = partial_args.merge(current_partial_args)
                    {
                        error_dump.append(error);
                    }
                },
                Err(error) => {
                    error_dump.append(error);
                },
            },
            _ => (),
        }
    }

    match partial_args.total() {
        Ok(arguments) if error_dump.errors().is_none() => {
            let name = arguments.name.value;
            let data = arguments.data.value;
            let quoted = quote! {
                impl ::webio::event::EventType for #typ {
                    type Data = #data;

                    fn name(&self) -> String {
                        #name.into()
                    }
                }
            };
            quoted.into_token_stream().into()
        },

        Ok(_) => error_dump.into_errors().unwrap().into_compile_error().into(),

        Err(errors) => {
            error_dump.append(errors);
            error_dump.into_errors().unwrap().into_compile_error().into()
        },
    }
}
