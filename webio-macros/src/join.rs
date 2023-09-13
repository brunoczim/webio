use syn::{
    parse::{Parse, ParseStream},
    token,
    Expr,
};

#[derive(Debug, Clone)]
pub struct Input {
    pub futures: Vec<Expr>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let futures = input.parse_terminated(Expr::parse, token::Comma)?;
        Ok(Self { futures: futures.into_iter().collect() })
    }
}
