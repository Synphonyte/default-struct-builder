use darling::{ast, util};
use darling::{FromDeriveInput, FromField};
use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::__private::TokenStream2;
use syn::spanned::Spanned;
use syn::{Error, Type};

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
    pub(crate) ty: syn::Type,
    pub(crate) attrs: Vec<syn::Attribute>,

    #[darling(default)]
    pub(crate) into: bool,

    #[darling(default)]
    pub(crate) skip: bool,
}

impl ToTokens for DefaultBuilderDeriveInput {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let DefaultBuilderDeriveInput {
            ref ident,
            ref data,
            ref generics,
        } = *self;

        let generic_idents: Vec<_> = generics.type_params().map(|t| &t.ident).collect();

        let type_params = &generics.params;
        let type_params = quote! { <#(#type_params),*> };

        let fields = data.as_ref().take_struct().expect("Is not enum").fields;

        let mut methods = vec![];

        for f in fields.clone().into_iter() {
            let name = f.ident.as_ref().expect("named field");

            if f.skip {
                continue;
            }

            let ty = &f.ty;
            let attrs = &f.attrs;

            if let Type::Path(p) = ty {
                if p.path.segments.len() == 1 {
                    let seg = p.path.segments.first().expect("Just checked path len");
                    if generic_idents.contains(&&seg.ident) {
                        if f.into {
                            tokens.extend(
                                Error::new_spanned(&f.ident, "Fields that have struct generic types currently don't support the `into` option")
                                    .to_compile_error(),
                            );
                            return;
                        }

                        let new_ident = format!("New__{}", &seg.ident);
                        let new_ident = syn::Ident::new(&new_ident, seg.span());
                        let new_ident_token = new_ident
                            .to_token_stream()
                            .into_iter()
                            .next()
                            .expect("should be one");

                        let old_ident_token = seg
                            .ident
                            .to_token_stream()
                            .into_iter()
                            .next()
                            .expect("should be one");

                        let replaced_type_params =
                            replace_in_stream(&type_params, &old_ident_token, &new_ident_token);

                        let replaced_where_clause =
                            generics.where_clause.as_ref().map(|where_clause| {
                                replace_in_stream(
                                    &where_clause.to_token_stream(),
                                    &old_ident_token,
                                    &new_ident_token,
                                )
                            });

                        let other_fields: Vec<_> = fields
                            .clone()
                            .into_iter()
                            .filter_map(|of| {
                                if of.ident == f.ident {
                                    None
                                } else {
                                    let ident = &of.ident;
                                    Some(quote! { #ident: self.#ident, })
                                }
                            })
                            .collect();

                        methods.push(quote! {
                            #(#attrs)*
                            #[allow(non_camel_case_types)]
                            pub fn #name<#new_ident>(self, value: #new_ident) -> #ident #replaced_type_params
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
            }

            if f.into {
                methods.push(quote! {
                    #(#attrs)*
                    pub fn #name<T__>(self, value: T__) -> Self
                    where
                        T__: Into<#ty>,
                    {
                        Self {
                            #name: value.into(),
                            ..self
                        }
                    }
                })
            } else {
                methods.push(quote! {
                    #(#attrs)*
                    pub fn #name(self, value: #ty) -> Self {
                        Self {
                            #name: value,
                            ..self
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
