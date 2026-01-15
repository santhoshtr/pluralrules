//! gen_rs is a Rust code generator for expression representations of CLDR plural rules.
use super::plural_category::PluralCategory;
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::collections::BTreeMap;
use std::str;
use unic_langid::LanguageIdentifier;

/// Generates the complete TokenStream for the generated Rust code. This wraps the head and tail of the .rs file around the generated CLDR expressions.
pub fn gen_fn(streams: BTreeMap<String, Vec<TokenStream>>, vr: &str) -> TokenStream {
    let ignore_noncritical_errors = quote! {
        #![allow(unused_variables, unused_parens)]
        #![cfg_attr(feature = "cargo-clippy", allow(clippy::float_cmp))]
        #![cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]
        #![cfg_attr(feature = "cargo-clippy", allow(clippy::nonminimal_bool))]
    };
    let use_statements = quote! {
        use super::operands::PluralOperands;
        use super::PluralCategory;
        use unic_langid::LanguageIdentifier;
        use unic_langid::subtags;
    };
    let langid_macro = quote! {
        macro_rules! langid {
            ($lang:expr, $script:expr, $region:expr) => {
                {
                    unsafe {
                        LanguageIdentifier::from_raw_parts_unchecked(
                            $lang,
                            $script,
                            $region,
                            None,
                        )
                    }
                }
            };
        }
    };
    let plural_function = quote! { pub type PluralRule = fn(&PluralOperands) -> PluralCategory; };
    let num: isize = vr.parse().unwrap();
    let ver = Literal::u64_unsuffixed(num as u64);
    let version = quote! { pub static CLDR_VERSION: usize = #ver; };
    let head = quote! { #ignore_noncritical_errors #use_statements #plural_function #version #langid_macro };
    let mut tokens = Vec::<TokenStream>::new();
    for (pr_type, stream) in streams {
        tokens.push(create_pr_type(&pr_type, stream));
    }
    let prs = quote! { #(#tokens)* };
    quote! { #head #prs }
}

// Function wraps all match statements for plural rules in a match for ordinal and cardinal rules
fn create_pr_type(pr_type: &str, streams: Vec<TokenStream>) -> TokenStream {
    let mut tokens = Vec::<TokenStream>::new();

    let match_name = match pr_type {
        "cardinal" => quote! { PRS_CARDINAL },
        "ordinal" => quote! { PRS_ORDINAL },
        _ => panic!("Unknown plural rule type"),
    };

    for func in &streams {
        tokens.push(func.clone());
    }
    quote! { pub const #match_name: &[(LanguageIdentifier, PluralRule)] = &[ #(#tokens),* ]; }
}

// Function wraps an expression in a match statement for plural category
fn create_return(cat: PluralCategory, exp: &TokenStream) -> TokenStream {
    match cat {
        PluralCategory::ZERO => quote! {if #exp { PluralCategory::ZERO } },
        PluralCategory::ONE => quote! {if #exp { PluralCategory::ONE } },
        PluralCategory::TWO => quote! {if #exp { PluralCategory::TWO } },
        PluralCategory::FEW => quote! {if #exp { PluralCategory::FEW } },
        PluralCategory::MANY => quote! {if #exp { PluralCategory::MANY } },
        PluralCategory::OTHER => quote! { { PluralCategory::OTHER } },
    }
}

/// Helper function to convert a string to its little-endian u64 representation for TinyStr8
fn str_to_u64(s: &str) -> u64 {
    let mut bytes = [0u8; 8];
    let s_bytes = s.as_bytes();
    bytes[..s_bytes.len()].copy_from_slice(s_bytes);
    u64::from_le_bytes(bytes)
}

/// Helper function to convert a string to its little-endian u32 representation for TinyStr4
fn str_to_u32(s: &str) -> u32 {
    let mut bytes = [0u8; 4];
    let s_bytes = s.as_bytes();
    bytes[..s_bytes.len()].copy_from_slice(s_bytes);
    u32::from_le_bytes(bytes)
}

pub fn gen_langid(id: &LanguageIdentifier) -> TokenStream {
    let (language, script, region, _) = id.clone().into_parts();

    // Language is always present (not optional) - takes u64
    let lang_str = language.as_str();
    let lang_raw = str_to_u64(lang_str);
    let lang = quote!(subtags::Language::from_raw_unchecked(#lang_raw));

    // Script is optional - takes u32
    let script = if let Some(script) = script {
        let script_str = script.as_str();
        let script_raw = str_to_u32(script_str);
        quote!(Some(subtags::Script::from_raw_unchecked(#script_raw)))
    } else {
        quote!(None)
    };

    // Region is optional - takes u32
    let region = if let Some(region) = region {
        let region_str = region.as_str();
        let region_raw = str_to_u32(region_str);
        quote!(Some(subtags::Region::from_raw_unchecked(#region_raw)))
    } else {
        quote!(None)
    };

    // No support for variants yet

    quote! {
        langid!(
            #lang,
            #script,
            #region
        )
    }
}

/// Generates the closures that comprise the majority of the generated rust code.
///
/// These statements are the expression representations of the CLDR plural rules.
pub fn gen_mid(
    lang: &LanguageIdentifier,
    pluralrule_set: &[(PluralCategory, TokenStream)],
) -> TokenStream {
    let langid = gen_langid(lang);
    // make pluralrule_set iterable
    let mut iter = pluralrule_set.iter();

    let queued = iter.next();
    let rule_tokens = match queued {
        Some(pair) => {
            // instantiate tokenstream for folded match rules
            let mut tokens = create_return(pair.0, &pair.1);

            // add all tokens to token stream, separated by commas
            for pair in iter {
                let condition = create_return(pair.0, &pair.1);
                tokens = quote! { #tokens else #condition };
            }
            tokens = quote! { #tokens else { PluralCategory::OTHER } };
            tokens
        }
        None => quote! { { PluralCategory::OTHER }  },
    };

    // We can't use a closure here because closures can't get rvalue
    // promoted to statics. They may in the future.
    quote! {(
        #langid,
        |po| {
            #rule_tokens
        }
    )}
}
