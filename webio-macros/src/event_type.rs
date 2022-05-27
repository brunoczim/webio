use crate::error;
use proc_macro2::{Ident, Span};
use syn::{
    parse::{Parse, ParseStream},
    token,
    Expr,
    Type,
};

#[derive(Debug, Clone)]
pub struct Argument<T> {
    pub key: Ident,
    pub value: T,
}

pub struct Arguments {
    pub name: Argument<Expr>,
    pub data: Argument<Type>,
}

#[derive(Debug, Clone, Default)]
pub struct PartialArguments {
    pub name: Option<Argument<Expr>>,
    pub data: Option<Argument<Type>>,
}

impl PartialArguments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: Self) -> syn::Result<()> {
        if self.name.is_some() {
            if let Some(argument) = other.name {
                Err(syn::Error::new(
                    argument.key.span(),
                    "setting already given",
                ))?;
            }
        } else {
            self.name = other.name;
        }

        if self.data.is_some() {
            if let Some(argument) = other.data {
                Err(syn::Error::new(
                    argument.key.span(),
                    "setting already given",
                ))?;
            }
        } else {
            self.data = other.data;
        }

        Ok(())
    }

    pub fn total(self) -> syn::Result<Arguments> {
        match self {
            PartialArguments { name: Some(name), data: Some(data) } => {
                Ok(Arguments { name, data })
            },

            arguments => {
                let mut error_dump = error::Dump::new();
                if arguments.name.is_none() {
                    error_dump.append(syn::Error::new(
                        Span::call_site(),
                        "event name is required, pass it as \
                         `#[event_type(name = \"foo\")]`",
                    ));
                }
                if arguments.data.is_none() {
                    error_dump.append(syn::Error::new(
                        Span::call_site(),
                        "event data type is required, pass it as \
                         `#[event_type(data = Foo)]`",
                    ));
                }
                Err(error_dump.into_errors().unwrap())
            },
        }
    }
}

impl Parse for PartialArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut this = Self::default();
        let mut has_comma = true;
        while !input.is_empty() {
            if !has_comma {
                Err(syn::Error::new(input.span(), "expected comma"))?;
            }

            let ident: Ident = input.parse()?;
            if ident == "name" {
                if this.name.is_some() {
                    Err(syn::Error::new(ident.span(), "setting already given"))?
                }
                let _: token::Eq = input.parse()?;
                this.name =
                    Some(Argument { key: ident, value: input.parse()? });
            } else if ident == "data" {
                if this.data.is_some() {
                    Err(syn::Error::new(ident.span(), "setting already given"))?
                }
                let _: token::Eq = input.parse()?;
                this.data =
                    Some(Argument { key: ident, value: input.parse()? });
            } else {
                Err(syn::Error::new(ident.span(), "unknown setting"))?
            }

            has_comma = false;
            if input.peek(token::Comma) {
                let _: token::Comma = input.parse()?;
                has_comma = true;
            }
        }
        Ok(this)
    }
}
