use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::{Comma, Semi},
    Expr,
};

struct JoinInput {
    futures: Vec<Expr>,
}

impl Parse for JoinInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let futures = input.parse_terminated::<Expr, Comma>(Expr::parse)?;
        Ok(Self { futures: futures.into_iter().collect() })
    }
}

/// Joins a list of futures, and returns their output into a tuple in the same
/// order that the futures were given. However, the future must be `'static`.
///
/// # Example
/// ```ignore
/// use std::time::Duration;
/// use webio::{join, task, time::timeout};
///
/// # fn main() {
/// # task::detach(async {
/// // Spawn some tasks.
/// let first_handle = task::spawn(async {
///     timeout(Duration::from_millis(50)).await;
///     3
/// });
/// let second_handle = task::spawn(async {
///     timeout(Duration::from_millis(60)).await;
///     5
/// });
/// let third_handle = task::spawn(async {
///     timeout(Duration::from_millis(40)).await;
///     7
/// });
///
/// // Join them.
/// let (first, second, third) = join!(first_handle, second_handle, third_handle);
///
/// // Expected output:
/// assert_eq!((first.unwrap(), second.unwrap(), third.unwrap()), (3, 5, 7));
/// # });
/// # }
/// ```
#[proc_macro]
pub fn join(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as JoinInput);
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

struct ConsoleInput {
    method: Ident,
    arguments: Vec<Expr>,
}

impl Parse for ConsoleInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = input.parse()?;
        let mut arguments = Vec::new();
        if input.peek(Semi) {
            input.parse::<Semi>()?;
            arguments.extend(
                input.parse_terminated::<Expr, Comma>(Expr::parse)?.into_iter(),
            );
        }
        Ok(Self { method, arguments })
    }
}

/// Prints to the JavaScript/browser/node console using a given method. Syntax:
/// ```ignore
/// console!($method; $($arguments),*)
/// ```
/// Each argument is converted into a `JsValue` using `Into`.
///
/// # Examples
///
/// ```ignore
/// use webio::console;
/// # fn main() {
/// console!(log; "Hello number", 5u8, "you're welcome!");
/// # }
/// ```
#[proc_macro]
pub fn console(raw_input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(raw_input as ConsoleInput);

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