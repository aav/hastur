use proc_macro::TokenStream;
use proc_macro2::{self};

use quote::quote;

use syn::parse_macro_input;

mod parse;

use parse::{Pattern, Receive};

fn pattern(pattern: Pattern) -> proc_macro2::TokenStream {
    let body = &pattern.body;

    let value = if let Some(type_pattern) = &pattern.type_pattern {
        quote! {
            __in.downcast::<#type_pattern>().unwrap()
        }
    } else {
        quote! {
            __in
        }
    };

    let assignment = if let Some(ident) = &pattern.ident {
        quote! {
            let #ident = #value;
        }
    } else {
        quote! {
            // no ident, no assignment needed
        }
    };

    let condition = if let Some(type_pattern) = &pattern.type_pattern {
        quote! {
            __in.is::<#type_pattern>()
        }
    } else {
        quote! {
            true
        }
    };

    quote! {
        if #condition {
            #assignment
            break #body;
        }
    }
}

#[proc_macro]
pub fn receive(input: TokenStream) -> TokenStream {
    let receive = parse_macro_input!(input as Receive);

    let prelude = quote! {
        use hastur;
        let mut __save_queue = hastur::SaveQueue::new();
    };

    let selective_save = quote! {
        else {
            __save_queue.push_front(__in);
            continue;
        }
    };

    let selective_restore = quote! {
        hastur::__selective_restore(__save_queue);
    };

    let patterns = receive.patterns.into_iter().map(pattern);

    let result = if let Some(after) = &receive.after {
        let duration = &after.duration;
        let body = &after.body;

        quote! {
            {
                #prelude;

                use async_std::future;
                use std::time::{Instant, Duration};

                let now = Instant::now();
                let __duration: Duration = #duration.into();

                let result =
                    loop {
                        let __in = future::timeout(__duration - now.elapsed(), hastur::__receive()).await;
                        match __in {
                            Err(_) => {
                                break {
                                    #body
                                };
                            },
                            Ok(__in) => {
                                #(#patterns)else* #selective_save
                            }
                        }
                    };

                #selective_restore;
                result
            }
        }
    } else {
        quote! {
            {
                #prelude;

                let result =
                    loop {
                        let __in = hastur::__receive().await;
                        #(#patterns)else* #selective_save
                    };

                #selective_restore;
                result
            }
        }
    };

    TokenStream::from(result)
}
