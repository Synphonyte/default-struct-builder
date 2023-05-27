use darling::{ast, util};
use darling::{FromDeriveInput, FromField};
use quote::{quote, ToTokens};
use syn::__private::TokenStream2;

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs(allow, doc, cfg))]
pub(crate) struct DefaultBuilderDeriveInput {
    pub(crate) ident: syn::Ident,
    pub(crate) data: ast::Data<util::Ignored, StructField>,
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
        } = *self;

        let fields = data.as_ref().take_struct().expect("Is not enum").fields;

        let mut methods = vec![];

        for f in fields.into_iter() {
            let name = f.ident.as_ref().expect("named field");

            if f.skip {
                continue;
            }

            let ty = &f.ty;
            let attrs = &f.attrs;

            if f.into {
                methods.push(quote! {
                    #(#attrs)*
                    pub fn #name<T>(self, value: T) -> Self
                    where
                        T: Into<#ty>,
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

        tokens.extend(quote! {
            impl #ident {
                #(#methods)*
            }
        });
    }
}
