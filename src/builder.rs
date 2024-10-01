use darling::ast::Generics;
use darling::{ast, util};
use darling::{FromDeriveInput, FromField};
use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use std::collections::{HashMap, HashSet};
use syn::__private::TokenStream2;
use syn::{Attribute, Error, GenericArgument, GenericParam, PathArguments, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs(allow, doc, cfg))]
pub(crate) struct DefaultBuilderDeriveInput {
    pub(crate) ident: syn::Ident,
    pub(crate) data: ast::Data<util::Ignored, StructField>,
    pub(crate) generics: ast::Generics<syn::GenericParam>,
}

#[derive(Debug, FromField)]
#[darling(attributes(builder), forward_attrs(allow, doc, cfg))]
pub(crate) struct StructField {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: Type,
    pub(crate) attrs: Vec<syn::Attribute>,

    #[darling(default)]
    pub(crate) into: bool,

    #[darling(default)]
    pub(crate) keep_outer: bool,

    #[darling(default)]
    pub(crate) skip: bool,

    #[darling(default)]
    pub(crate) keep_type: bool,
}

impl ToTokens for DefaultBuilderDeriveInput {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let DefaultBuilderDeriveInput {
            ref ident,
            ref data,
            ref generics,
        } = *self;

        let generic_idents: Vec<_> = generics.type_params().map(|t| &t.ident).collect();

        let key_depends_on_value = find_dependencies(&generics, &generic_idents);

        let type_params = &generics.params;
        let type_params = quote! { <#(#type_params),*> };

        let fields = data.as_ref().take_struct().expect("Is not enum").fields;

        let mut methods = vec![];

        let dot_dot_self = if fields.len() == 1 {
            quote! {}
        } else {
            quote! { ..self }
        };

        for f in fields.clone().into_iter() {
            let name = f.ident.as_ref().expect("named field");

            if f.skip {
                continue;
            }

            let ty = &f.ty;
            let attrs = &f.attrs;

            let mut new_idents = vec![];
            let mut old_new_ident_tokens = vec![];
            let empty = HashSet::new();

            if !f.keep_type {
                let mut generic_field = false;

                for generic_ident in generic_idents.iter() {
                    if stream_contains(
                        &ty.to_token_stream(),
                        &generic_ident
                            .to_token_stream()
                            .into_iter()
                            .next()
                            .expect("should be one"),
                    ) {
                        generic_field = true;

                        if f.into {
                            tokens.extend(
                                Error::new_spanned(&f.ident, "Fields that have struct generic types currently don't support the `into` option")
                                    .to_compile_error(),
                            );
                            return;
                        }

                        let (new_ident, new_ident_token) =
                            create_new_ident_and_token(generic_ident);

                        let old_ident_token = generic_ident
                            .to_token_stream()
                            .into_iter()
                            .next()
                            .expect("should be one");

                        new_idents.push(new_ident.clone());
                        old_new_ident_tokens.push((old_ident_token, new_ident_token));

                        for ident in key_depends_on_value
                            .get(&generic_ident.to_string())
                            .unwrap_or(&empty)
                            .iter()
                        {
                            let (new_ident, new_ident_token) = create_new_ident_and_token(ident);
                            new_idents.push(new_ident);
                            old_new_ident_tokens.push((
                                ident
                                    .to_token_stream()
                                    .into_iter()
                                    .next()
                                    .expect("should be one"),
                                new_ident_token,
                            ));
                        }
                    }
                }

                if generic_field {
                    let mut replaced_type_params = type_params.clone();

                    for (old_ident_token, new_ident_token) in old_new_ident_tokens.iter() {
                        replaced_type_params = replace_in_stream(
                            &replaced_type_params.clone(),
                            old_ident_token,
                            new_ident_token,
                        );
                    }

                    let replaced_where_clause =
                        generics.where_clause.as_ref().map(|where_clause| {
                            let mut replaced_stream = where_clause.to_token_stream();

                            for (old_ident_token, new_ident_token) in old_new_ident_tokens.iter() {
                                replaced_stream = replace_in_stream(
                                    &replaced_stream.clone(),
                                    old_ident_token,
                                    new_ident_token,
                                )
                            }

                            replaced_stream
                        });

                    let mut replaced_field_type = ty.to_token_stream();

                    for (old_ident_token, new_ident_token) in old_new_ident_tokens.iter() {
                        replaced_field_type = replace_in_stream(
                            &replaced_field_type.clone(),
                            old_ident_token,
                            new_ident_token,
                        );
                    }

                    let other_fields: Vec<_> = fields
                        .clone()
                        .into_iter()
                        .filter_map(|of| {
                            if of.ident == f.ident {
                                None
                            } else {
                                let ident = &of.ident;
                                let mut token_stream = quote! { #ident: self.#ident, };

                                if let Type::Path(path) = &of.ty {
                                    if let Some(seg) = path.path.segments.last() {
                                        if seg.ident == "PhantomData" {
                                            token_stream =
                                                quote!( #ident: std::marker::PhantomData, );
                                        }
                                    }
                                }

                                Some(token_stream)
                            }
                        })
                        .collect();

                    methods.push(quote! {
                                #(#attrs)*
                                #[allow(non_camel_case_types)]
                                pub fn #name<#(#new_idents),*>(self, value: #replaced_field_type) -> #ident #replaced_type_params
                                #replaced_where_clause
                                {
                                    #ident::#replaced_type_params {
                                        #name: value,
                                        #(#other_fields)*
                                    }
                                }
                            });

                    continue;
                }
            }

            let unwrap_inner_type = [
                (get_inner_type(ty, "Box"), quote! { Box }),
                (get_inner_type(ty, "Rc"), quote! { Rc }),
                (get_inner_type(ty, "Arc"), quote! { Arc }),
            ]
            .into_iter()
            .filter_map(|(ty, tok)| ty.map(|t| (t, tok)))
            .next();

            let option_inner_type = get_inner_type(ty, "Option");

            if f.into {
                if let Some(inner_type) = option_inner_type {
                    methods.push(quote! {
                        #(#attrs)*
                        pub fn #name<OptionInnerType>(self, value: impl Into<Option<OptionInnerType>>) -> Self
                        where
                            OptionInnerType: Into<#inner_type>
                        {
                            Self {
                                #name: value.into().map(|v| v.into()),
                                #dot_dot_self
                            }
                        }
                    })
                } else {
                    methods.push(quote! {
                        #(#attrs)*
                        pub fn #name(self, value: impl Into<#ty>) -> Self {
                            Self {
                                #name: value.into(),
                                #dot_dot_self
                            }
                        }
                    })
                }
            } else if unwrap_inner_type.is_some() && !f.keep_outer {
                let (inner_type, inner_type_token) = unwrap_inner_type.expect("just checked above");

                auto_wrapper_method(
                    &mut methods,
                    &dot_dot_self,
                    name,
                    attrs,
                    inner_type,
                    inner_type_token,
                );
            } else {
                methods.push(quote! {
                    #(#attrs)*
                    pub fn #name(self, value: #ty) -> Self {
                        Self {
                            #name: value,
                            #dot_dot_self
                        }
                    }
                });
            }
        }

        let where_clause = generics
            .where_clause
            .as_ref()
            .map(|w| quote! { #w })
            .unwrap_or(quote! {});

        tokens.extend(quote! {
            impl #type_params #ident #type_params #where_clause {
                #(#methods)*
            }
        });
    }
}

fn find_dependencies(
    generics: &&Generics<GenericParam>,
    generic_idents: &[&Ident],
) -> HashMap<String, HashSet<Ident>> {
    let mut key_depends_on_value: HashMap<String, HashSet<Ident>> = HashMap::new();

    for param in generics.type_params() {
        let lhs = &param.ident;

        for ident in generic_idents.iter() {
            let token = ident
                .to_token_stream()
                .into_iter()
                .next()
                .expect("should be one");

            if stream_contains(&param.bounds.to_token_stream(), &token) {
                key_depends_on_value
                    .entry(lhs.to_string())
                    .or_default()
                    .insert((*ident).clone());
            }
        }
    }

    if let Some(where_clause) = &generics.where_clause {
        for predicate in where_clause.predicates.iter() {
            if let syn::WherePredicate::Type(type_predicate) = predicate {
                let bounded = type_predicate.bounded_ty.to_token_stream();

                for lhs in generic_idents.iter() {
                    if stream_contains(
                        &bounded,
                        &lhs.to_token_stream()
                            .into_iter()
                            .next()
                            .expect("should be one"),
                    ) {
                        let bounds = type_predicate.bounds.to_token_stream();

                        for rhs in generic_idents.iter() {
                            let rhs_token = rhs
                                .to_token_stream()
                                .into_iter()
                                .next()
                                .expect("should be one");

                            if stream_contains(&bounds, &rhs_token) {
                                key_depends_on_value
                                    .entry(lhs.to_string())
                                    .or_default()
                                    .insert((*rhs).clone());
                            }
                        }
                    }
                }
            }
        }
    }

    let mut new_dependency_found = true;

    // find cascading dependencies
    while new_dependency_found {
        new_dependency_found = false;

        for (lhs, rhs) in key_depends_on_value.clone().iter() {
            let mut new_rhs = rhs.clone();

            for rhs in rhs.iter() {
                if let Some(rhs_rhs) = key_depends_on_value.get(&rhs.to_string()) {
                    for rhs_rhs in rhs_rhs.iter() {
                        new_rhs.insert(rhs_rhs.clone());
                    }
                }
            }

            if new_rhs.len() > rhs.len() {
                new_dependency_found = true;

                key_depends_on_value.insert(lhs.clone(), new_rhs);
            }
        }
    }

    key_depends_on_value
}

fn create_new_ident_and_token(old_ident: &Ident) -> (Ident, TokenTree) {
    let new_ident = format!("New__{}", old_ident);
    let new_ident = syn::Ident::new(&new_ident, old_ident.span());
    let new_ident_token = new_ident
        .to_token_stream()
        .into_iter()
        .next()
        .expect("should be one");
    (new_ident, new_ident_token)
}

fn replace_old_tree_with_new(
    t: TokenTree,
    old_ident_token: &TokenTree,
    new_ident_token: &TokenTree,
) -> TokenTree {
    if format!("{t}") == format!("{old_ident_token}") {
        new_ident_token.clone()
    } else if let TokenTree::Group(ref g) = t {
        TokenTree::Group(Group::new(
            g.delimiter(),
            replace_in_stream(&g.stream(), old_ident_token, new_ident_token),
        ))
    } else {
        t
    }
}

fn replace_in_stream(
    s: &TokenStream,
    old_ident_token: &TokenTree,
    new_ident_token: &TokenTree,
) -> TokenStream {
    TokenStream::from_iter(
        s.clone()
            .into_iter()
            .map(|t| replace_old_tree_with_new(t, old_ident_token, new_ident_token)),
    )
}

fn stream_contains(s: &TokenStream, t: &TokenTree) -> bool {
    s.clone().into_iter().any(|token| {
        if token.to_string() == t.to_string() {
            return true;
        }

        if let TokenTree::Group(ref g) = token {
            return stream_contains(&g.stream(), t);
        }

        false
    })
}

fn get_inner_type(ty: &Type, outer_type_start: &str) -> Option<Type> {
    match ty {
        Type::Path(path) => {
            if let Some(seg) = path.path.segments.last() {
                if seg.ident.to_string().starts_with(outer_type_start) {
                    if let PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(GenericArgument::Type(ty)) = args.args.first() {
                            return Some(ty.clone());
                        }
                    }
                }
            }

            None
        }
        _ => None,
    }
}

fn auto_wrapper_method(
    methods: &mut Vec<TokenStream>,
    dot_dot_self: &TokenStream,
    name: &Ident,
    attrs: &Vec<Attribute>,
    inner_type: Type,
    wrapper_type: TokenStream,
) {
    let inner_type = if let Type::TraitObject(obj) = inner_type {
        let bounds = obj.bounds;
        quote! { impl #bounds + 'static }
    } else {
        inner_type.to_token_stream()
    };

    methods.push(quote! {
        #(#attrs)*
        pub fn #name(self, value: #inner_type) -> Self {
            Self {
                #name: #wrapper_type::new(value),
                #dot_dot_self
            }
        }
    });
}
