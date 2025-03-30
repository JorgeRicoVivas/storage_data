//! Refer to the [storage_data](https://crates.io/crates/storage_data) crate, don't use this crate
//! independently of it.
//!
//! This ``storage_data`` crate allows to easily associate Local/Session storage data through the
//! StorageData struct and to retrieve and set the value without requiring to manually interacting
//! with the Web Storage API, and this crate is made to allow creating a struct where associating
//! multiple StorageData is made much simpler and maintainable.
extern crate alloc;
extern crate proc_macro;

use crate::error_messages::ErrorMessages;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use convert_case::Casing;
use core::mem;
use proc_macro::{TokenStream, TokenTree};
use proc_macro2::{Group, Span};
use proc_macro_error::{Diagnostic, Level};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Expr, Lit, LitStr, Meta, Visibility};

pub(crate) mod error_messages;


/// This eases up manually calling the Web Storage API, but this might still feel uncomfortable to
/// use, as you need to manually associate every value in a StorageData, to alleviate this, the
/// [crate::WebStorage] derive macro allows you to create a struct where you define a group of storage
/// values, and it modifies said struct to turn every value into a StorageData.
///
/// For example, if you have these values you want to associate to a Storage:
///
/// - ``visited_times: usize``: Containing the times the user, visits a page.
/// - ``picked_products: Vec<String>``: A list of products the user picked for buying, which is a
///    value that should be stored in a Session Storage rather than Local.
/// - ``user_info: UserInfo``: A custom-made struct with personal information about the user.
///
/// You could define a storage such as this:
///
/// ```rust no_compile
/// use derive_web_storage::WebStorage;
///
/// #[derive(Debug)]
/// #[WebStorage(
///     // Optional: This prepends every Key of every storage data with 'USER::_::ALT::_'.
///     Prepend_keys_with(USER::_::ALT::_),
///     // Optional: This changes the default visibility in which this WebStorage can be created.
///     ConstructorVisibility(pub(crate)),
///     // Optional: This changes the default storage used in every StorageData.
///     // Default: The one you set in the features when importing this crate; If you didn't set
///     //          a default Storage, LocalStorage will be used.
///     StorageKind(Local)
/// )]
/// pub struct Storage {
///     // It isn't necessary to specify the default value for visited_times as 'usize' implements
///     // Default.
///     visited_times: usize,
///
///     // This is only saved as a Session value, instead of Local.
///     #[StorageKind(Session)]
///     picked_products: Vec<String>,
///
///     // This value needs to use a default constructor, as UserInfo doesn't have one.
///     // In a real case scenario this should be an Option initialized as None.
///     #[default(UserInfo {
///         name: "Jorge Rico Vivas".to_string(),
///         preferred_color: "Blue".to_string()
///     })]
///     user_info: UserInfo,
/// }
///
/// struct UserInfo{
///     name:String, preferred_color: String,
/// }
///
/// let mut storage = Storage::new();
///
/// // Increment visited times.
/// *storage.visited_times+=1;
///
/// // Print info about the user.
/// println!("The user {} has visited this page {} time(s)",
///     storage.user_info.name, storage.visited_times);
///
/// // Storage is saved here when dropped, you aren't required to manually save it.
/// ```
#[allow(non_snake_case)]
#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn WebStorage(macro_attr: TokenStream, input: TokenStream) -> TokenStream {
    let DeriveInput {
        vis,
        attrs: _attrs,
        ident: struct_ident,
        data,
        ..
    } = parse_macro_input!(input as DeriveInput);
    let struct_data = match data {
        Data::Struct(struct_data) => struct_data,
        _ => {
            ErrorMessages::ExpectedDifferent {
                expected: "Struct",
                span: struct_ident.span(),
                found: struct_ident.to_token_stream(),
            }
            .abort();
        }
    };

    let first_field_is_named = struct_data
        .fields
        .iter()
        .next()
        .map(|first_field| first_field.ident.is_some())
        .unwrap_or(true);
    if !first_field_is_named {
        ErrorMessages::StructFieldsMustBeNamed {
            fields: struct_data.fields,
        }
        .abort();
    }

    let attrs = separate_token_stream_by_commas(macro_attr)
        .into_iter()
        .map(|attr| ident_and_group(attr))
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    let mut constructor_visibility = Some(vis.clone());
    let mut prepend = String::new();
    let mut storage_kind = quote! {};

    #[cfg(feature = "default_storage_local")]
    let mut storage_kind_for_doc = Lit::Str(LitStr::new("Local", Span::call_site()));
    #[cfg(feature = "default_storage_session")]
    let mut storage_kind_for_doc = Lit::Str(LitStr::new("Session", Span::call_site()));

    for (ident, contents) in attrs {
        match ident.to_string().to_lowercase().trim() {
            "prepend" | "prepend_key" | "prepend_keys" | "prepend_with" | "prepend_key_with"
            | "prepend_keys_with" => {
                prepend = group_interior(contents).to_string().replace(" ", "");
            }
            "constructorvisibility"
            | "constructor_visibility"
            | "constructorvis"
            | "constructor_vis"
            | "newvisibility"
            | "new_visibility"
            | "newvis"
            | "new_vis" => {
                let visibility = syn::parse::<Visibility>(group_interior(contents.clone()));
                if visibility.is_err() {
                    let contents: proc_macro2::TokenStream = contents.into();
                    ErrorMessages::ExpectedDifferent {
                        expected: "a visibility",
                        span: contents.span(),
                        found: contents,
                    }
                    .abort()
                }
                constructor_visibility = Some(visibility.unwrap());
            }
            "storage_kind" | "storagekind" | "storage" => {
                let contents = proc_macro2::TokenStream::from(group_interior(contents));
                storage_kind_for_doc =
                    Lit::Str(LitStr::new(&*contents.to_string(), contents.span()));
                storage_kind =
                    quote! { with storage kind ::storage_data::StorageKind:: #contents, };
            }
            _ => {}
        }
    }
    let constructor_visibility = constructor_visibility.unwrap_or(vis.clone());

    let fields_count = struct_data.fields.len();

    let mut fields_tokens = quote! {};
    struct_data.fields.iter().for_each(|field| {
        let variable_name = field.ident.as_ref().unwrap();
        let variable_type = &field.ty;
        let variable_doc = field
            .attrs
            .iter()
            .map(|attr| extract_doc_comment(attr))
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect::<Vec<_>>()
            .join("\n");

        let separated_attributes = field
            .attrs
            .iter()
            .map(|attr| {
                let tokens = match &attr.meta {
                    Meta::Path(path) => {
                        panic!()
                    }
                    Meta::List(list) => list.tokens.clone(),
                    Meta::NameValue(name_value) => name_value.value.to_token_stream(),
                };
                (
                    syn::parse::<proc_macro2::Ident>(attr.path().segments.to_token_stream().into())
                        .ok(),
                    proc_macro2::TokenStream::from(group_interior(tokens.into())),
                )
            })
            .filter(|(ident, _)| ident.is_some())
            .map(|(ident, tokens)| (ident.unwrap(), tokens));

        let mut field_storage = None;
        let mut field_storage_kind_for_doc = None;
        for (ident, contents) in separated_attributes {
            match ident.to_string().to_lowercase().trim() {
                "storage_kind" | "storagekind" | "storage" => {
                    field_storage_kind_for_doc = Some(Lit::Str(LitStr::new(
                        &*contents.to_string(),
                        contents.span(),
                    )));
                    field_storage =
                        Some(quote! { with storage kind ::storage_data::StorageKind:: #contents, });
                }
                _ => {}
            }
        }

        let field_storage = field_storage.unwrap_or(storage_kind.clone());
        let field_storage_kind_for_doc =
            field_storage_kind_for_doc.unwrap_or(storage_kind_for_doc.clone());

        let web_name = format!(
            "{prepend}{}",
            variable_name.to_string().to_case(convert_case::Case::Camel)
        );
        let default_field = field
            .attrs
            .iter()
            .filter(|attr| attr.path().to_token_stream().to_string().eq("default"))
            .next();
        let default = match default_field {
            None => quote! {Default::default()},
            Some(attr) => {
                match &attr.meta {
                    Meta::Path(path) => {
                        unreachable!()
                    }
                    Meta::List(list) => list.tokens.clone(),
                    Meta::NameValue(name_value) => name_value.value.to_token_stream(),
                }
            }
        };
        fields_tokens = quote! {
            #fields_tokens
            {
                variable #variable_name,
                type #variable_type,
                named #web_name,
                default { #default },
                #field_storage
                with documentation #variable_doc,
                storage kind for doc #field_storage_kind_for_doc,
            }
        };
    });

    let res = quote! {
        ::storage_data::define_storage!{
            #vis #struct_ident with storage data {
                len: #fields_count,
                constructor visibility: #constructor_visibility,
                #fields_tokens
            }
        }
    };
    res.into()
}

fn separate_token_stream_by_commas(token_stream: TokenStream) -> Vec<TokenStream> {
    let mut all_token_streams = Vec::new();
    let mut current_token_streams = proc_macro::TokenStream::new();
    token_stream.into_iter().for_each(|token| {
        let is_separator = match &token {
            proc_macro::TokenTree::Punct(punct) => punct.as_char() == ',',
            _ => false,
        };
        if is_separator {
            all_token_streams.push(mem::take(&mut current_token_streams));
        } else {
            current_token_streams.extend(TokenStream::from(token));
        }
    });
    if !current_token_streams.is_empty() {
        all_token_streams.push(mem::take(&mut current_token_streams));
    }
    all_token_streams
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>()
}

fn ident_and_group(input: TokenStream) -> Option<(proc_macro2::Ident, proc_macro::TokenStream)> {
    let ident;
    let group;
    let mut token_iter = input.into_iter();
    match token_iter.next() {
        None => return None,
        Some(token) => match &token {
            proc_macro::TokenTree::Ident(_) => {
                ident = token;
            }
            _ => return None,
        },
    }
    match token_iter.next() {
        None => return None,
        Some(token) => match &token {
            proc_macro::TokenTree::Group(_) => {
                group = TokenStream::from(token);
            }
            _ => return None,
        },
    }
    match token_iter.next() {
        Some(_) => None,
        None => {
            let ident = syn::parse::<proc_macro2::Ident>(TokenStream::from(ident)).unwrap();
            Some((ident, group))
        }
    }
}

fn group_interior(token_stream: TokenStream) -> TokenStream {
    let next = token_stream.clone().into_iter().next();
    if next.is_none() {
        return token_stream;
    }
    match next.unwrap() {
        TokenTree::Group(group) => group.stream(),
        _ => token_stream,
    }
}

fn extract_doc_comment(attr: &syn::Attribute) -> Option<String> {
    // Check if the attribute is a `doc` attribute
    if attr.path().is_ident("doc") {
        // Parse the attribute tokens into a string
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if let Expr::Lit(expr_lit) = &meta.value {
                if let syn::Lit::Str(lit) = &expr_lit.lit {
                    return Some(lit.value());
                }
            }
        }
    }
    None
}
