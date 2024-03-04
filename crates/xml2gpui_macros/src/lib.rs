extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Expr, Ident, LitStr, Token,
};

// Updated to hold vectors of vectors to represent groups of tailwind classes
struct TailwindToGpuiInput {
    element_name: Ident,
    class_name: Ident,
    tailwind_class_groups: Vec<Vec<LitStr>>,
    default_case: Box<Expr>,
}

impl Parse for TailwindToGpuiInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let element_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let class_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut tailwind_class_groups = Vec::new();

        while !input.peek(Token![_]) {
            let content;
            // Correctly parse the bracketed group
            syn::bracketed!(content in input);
            let classes = Punctuated::<LitStr, Comma>::parse_terminated(&content)?;

            tailwind_class_groups.push(classes.into_iter().collect());

            // Optionally consume a comma after the group
            let _ = input.parse::<Token![,]>().ok();
        }

        // Consume the arrow before the default case
        input.parse::<Token![_]>()?;
        input.parse::<Token![=>]>()?;

        let default_case: Expr = input.parse()?;

        Ok(TailwindToGpuiInput {
            element_name,
            class_name,
            tailwind_class_groups,
            default_case: Box::new(default_case),
        })
    }
}
#[proc_macro]
pub fn tailwind_to_gpui(input: TokenStream) -> TokenStream {
    let TailwindToGpuiInput {
        element_name,
        class_name,
        tailwind_class_groups,
        default_case,
    } = parse_macro_input!(input as TailwindToGpuiInput);

    let tailwind_matches = tailwind_class_groups.iter().flat_map(|group| {
        group.iter().map(|class| {
            // Replace "-" to "_" and "/" to "_" in class name
            let method_name = Ident::new(
                class
                    .value()
                    .replace("-", "_")
                    .replace("/", "_")
                    .replace(".", "p")
                    .as_str(),
                class.span(),
            );

            // Fonts are little bit different
            if class.value().starts_with("font-") {
                let font_weight = Ident::new(match class.value().replace("font-", "").as_str() {
                    "thin" => "THIN",
                    "extralight" => "EXTRA_LIGHT",
                    "light" => "LIGHT",
                    "normal" => "NORMAL",
                    "medium" => "MEDIUM",
                    "semibold" => "SEMIBOLD",
                    "bold" => "BOLD",
                    "extrabold" => "EXTRA_BOLD",
                    "black" => "BLACK",
                    _ => "NORMAL",
                }, class.span());
                quote! {
                    #class => #element_name.font_weight(FontWeight::#font_weight),
                }
            } else {
                quote! {
                    #class => #element_name.#method_name(),
                }
            }
        })
    });

    let expanded = quote! {
        match #class_name {
            #(#tailwind_matches)*
            _ => #default_case
        }
    };

    TokenStream::from(expanded)
}
