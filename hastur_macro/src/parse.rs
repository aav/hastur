use syn::parse::{Error, Parse, ParseStream, Result};

use syn::{custom_keyword, Block, Expr, Ident, Token, TypePath};

#[derive(Debug)]
pub(crate) struct Pattern {
    pub ident: Option<Ident>,
    pub type_pattern: Option<TypePath>,
    pub body: Block,
}

#[derive(Debug)]
pub(crate) struct After {
    pub duration: Expr,
    pub body: Block,
}

#[derive(Debug)]
pub(crate) struct Receive {
    pub patterns: Vec<Pattern>,
    pub after: Option<After>,
}

fn body(input: &mut ParseStream) -> Result<Block> {
    input.parse::<Token![=>]>()?;
    input.parse::<Block>()
}

fn parse_pattern(input: &mut ParseStream) -> Result<Pattern> {
    let lookahead = input.lookahead1();
    let ident = if lookahead.peek(Token![_]) {
        input.parse::<Token![_]>()?;
        None
    } else {
        Some(input.parse::<Ident>()?)
    };

    input.parse::<Token![:]>()?;

    let lookahead = input.lookahead1();
    let type_pattern = if lookahead.peek(Token![_]) {
        input.parse::<Token![_]>()?;
        None
    } else {
        let type_pattern = input.parse::<TypePath>()?;
        Some(type_pattern)
    };

    let body = body(input)?;

    Ok(Pattern {
        ident,
        type_pattern,
        body,
    })
}

fn parse_after(input: &mut ParseStream) -> Result<After> {
    let duration = input.parse::<Expr>()?;
    let body = body(input)?;

    Ok(After { duration, body })
}

impl Parse for Receive {
    fn parse(mut input: ParseStream) -> Result<Self> {
        let mut after = None;
        let mut patterns = Vec::new();

        mod custom {
            super::custom_keyword!(after);
        }

        while !input.is_empty() {
            if input.lookahead1().peek(custom::after) {
                let span = input.parse::<custom::after>()?.span;
                after = Some(parse_after(&mut input)?);

                if input.lookahead1().peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }

                if !input.is_empty() {
                    return Err(Error::new(span, "'after' is not last"));
                }
            } else {
                let pattern = parse_pattern(&mut input)?;
                patterns.push(pattern);
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Receive { patterns, after })
    }
}
