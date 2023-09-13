use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    token,
    Expr,
};

#[derive(Debug, Clone)]
pub struct Input {
    pub method: Ident,
    pub arguments: Vec<Expr>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = input.parse()?;
        let mut arguments = Vec::new();
        if input.peek(token::Semi) {
            input.parse::<token::Semi>()?;
            arguments.extend(
                input.parse_terminated(Expr::parse, token::Comma)?.into_iter(),
            );
        }
        Ok(Self { method, arguments })
    }
}
