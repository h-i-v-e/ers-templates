use rusty_hbs_helpers::hbs_parser::Compiler;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, Ident, Result, LitStr, Token};
use syn::spanned::Spanned;

struct TemplateArgs{
    src: Option<String>
}

impl Parse for TemplateArgs{
    fn parse(input: ParseStream) -> Result<Self> {
        let mut src : Option<String> = None;
        loop {
            let ident = input.parse::<Ident>()?;
            let label = ident.to_string();
            input.parse::<Token!(=)>()?;
            if label == "path"{
                src = Some(input.parse::<LitStr>()?.value());
            }
            if input.is_empty(){
                break;
            }
            input.parse::<Token!(,)>()?;
        }
        Ok(TemplateArgs{
            src
        })
    }
}

struct DisplayParts{
    name: Ident,
    lifetimes: proc_macro2::TokenStream,
    content: Option<proc_macro2::TokenStream>
}

impl Parse for DisplayParts{
    fn parse(input: ParseStream) -> Result<Self> {
        let input = input.parse::<DeriveInput>()?;
        let lifetimes = input.generics.into_token_stream();
        let name = input.ident;
        let attr = match input.attrs.get(0){
            None => return Ok(Self{
                name, lifetimes, content: None
            }),
            Some(attr) => attr
        };
        let args = attr.parse_args::<TemplateArgs>()?;
        let src = match args.src{
            None => return Ok(Self{
                name, lifetimes, content: None
            }),
            Some(src) => src
        };
        let src = match std::fs::read_to_string(&src) {
            Ok(src) => src,
            Err(err) => {
                let path = std::fs::canonicalize(std::path::Path::new("./")).unwrap();
                return Err(
                    syn::Error::new(
                        attr.span(),
                        format!(
                            "unable to read {}, {}", path.join(src).to_str().unwrap(), err.to_string()
                        )
                    )
                )
            }
        };
        Ok(Self{
          name, lifetimes,
            content: Some(proc_macro2::token_stream::TokenStream::from_str(Compiler::new().compile(&src).as_str()).unwrap())
        })
    }
}

#[proc_macro_derive(Renderable, attributes(Template))]
pub fn make_renderable(raw: TokenStream) -> TokenStream{
    let DisplayParts{
        name, lifetimes, content
    } = parse_macro_input!(raw as DisplayParts);
    TokenStream::from(match content {
        Some(content) => quote! {
            impl #lifetimes std::fmt::Display for #name #lifetimes {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    #content
                    Ok(())
                }
            }
        },
        None => quote! {
            impl #lifetimes std::fmt::Display for #name #lifetimes {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    Ok(())
                }
            }
        }
    })
}
