use syn::{
    parse::{Parse, ParseStream},
    token,
    Expr,
    Pat,
};

#[derive(Debug, Clone)]
pub struct Arm {
    pub pattern: Pat,
    pub future: Expr,
    pub output: Expr,
}

impl Parse for Arm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pattern = input.parse()?;
        input.parse::<token::Eq>()?;
        let future = input.parse()?;
        input.parse::<token::FatArrow>()?;
        let output = input.parse()?;
        Ok(Self { pattern, future, output })
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub arms: Vec<Arm>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arms = input.parse_terminated::<Arm, token::Comma>(Arm::parse)?;
        Ok(Self { arms: arms.into_iter().collect() })
    }
}
