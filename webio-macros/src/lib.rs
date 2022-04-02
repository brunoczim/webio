use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Comma,
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
