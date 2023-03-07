use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::borrow::Cow;
use std::fmt::Write;
use std::str::FromStr;
use regex::{Captures, Regex};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, Ident, Result, LitStr, Token};
use syn::spanned::Spanned;

fn escape<'a>(reg: &'a Regex, content: &'a str) -> Cow<'a, str> {
    reg.replace_all(
        &content, |captures: &Captures| format!("\\{}", &captures[0])
    )
}

fn write_str(reg: &Regex, out: &mut String, content: &str) {
    if content.is_empty(){
        return;
    }
    out.write_str("f.write_str(\"").unwrap();
    out.write_str(escape(reg, content).as_ref()).unwrap();
    out.write_str("\")?;").unwrap();
}

fn parse_template(content: & str, open: &str, close: &str)
    -> Result<proc_macro2::TokenStream> {
    let esc = Regex::new(r"[?.\\$^]").unwrap();
    let reg = Regex::new(format!(
        "{}=?.+?{}",
        escape(&esc, open).as_ref(),
        escape(&esc, close).as_ref()
    ).as_ref()).unwrap();
    let clean_reg = Regex::new("[\\\\\"]").unwrap();
    let mut start : usize = 0;
    let mut out = String::new();
    for mtch in reg.find_iter(content){
        write_str(&clean_reg, &mut out, &content[start..mtch.start()]);
        if &content[mtch.start() + open.len()..mtch.start() + open.len() + 1] == "="{
            out.write_str("Display::fmt(&(").unwrap();
            out.write_str(&content[mtch.start() + open.len() + 1 .. mtch.end() - close.len()]).unwrap();
            out.write_str("),f)?;").unwrap();
        }
        else{
            out.write_str(&content[mtch.start() + open.len() .. mtch.end() - close.len()]).unwrap();
        }
        start = mtch.end();
    }
    write_str(&clean_reg, &mut out, &content[start..]);
    Ok(proc_macro2::token_stream::TokenStream::from_str(out.as_str())?)
}

struct TemplateArgs{
    src: Option<String>,
    open_with: String,
    close_with: String
}

impl Parse for TemplateArgs{
    fn parse(input: ParseStream) -> Result<Self> {
        let mut src : Option<String> = None;
        let mut open_with: Option<String> = None;
        let mut close_with: Option<String> = None;
        loop {
            let ident = input.parse::<Ident>()?;
            let label = ident.to_string();
            input.parse::<Token!(=)>()?;
            if label == "path"{
                src = Some(input.parse::<LitStr>()?.value());
            }
            else if label == "open"{
                open_with = Some(input.parse::<LitStr>()?.value());
            }
            else if label == "close"{
                close_with = Some(input.parse::<LitStr>()?.value());
            }
            else{
                return Err(syn::Error::new(
                    Span::from(ident.span().unwrap()), "Invalid parameter, expected one of path, open or close"
                ))
            }
            if input.is_empty(){
                break;
            }
            input.parse::<Token!(,)>()?;
        }
        Ok(TemplateArgs{
            src,
            open_with: open_with.unwrap_or("<?%".to_string()),
            close_with: close_with.unwrap_or("%>".to_string()),
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
            content: Some(parse_template(
                src.as_str(),
                args.open_with.as_str(),
                args.close_with.as_str()
            )?)
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

#[cfg(test)]
mod tests {
    use crate::parse_template;

    #[test]
    fn it_works() {
        println!("{}", parse_template(
            "werty $=self.test1^ $ for i in 0..5{ ^$=i^$}^",
        "$", "^").unwrap())
    }
}
