//! Generates builder methods of every field of a struct. It is meant to be used on structs that
//! implement `Default`. There is no separate builder struct generated and no need to call a
//! `build()` method at the end or `.unwrap()`.
//!
//! This crate is used by the crate `leptos-use` for the option structs that
//! can be passed to the various functions.
//!
//! ## Installation
//!
//! In your project folder run
//!
//! ```sh
//! cargo add default-struct-builder
//! ```
//!
//! ## Usage
//!
//! It is very easy to use:
//!
//! ```
//! use default_struct_builder::DefaultBuilder;
//!
//! #[derive(DefaultBuilder, Default)]
//! pub struct SomeOptions {
//!     throttle: f64,
//!
//!     #[builder(into)]
//!     offset: Option<f64>,
//!
//!     #[builder(skip)]
//!     not_included: u32,
//! }
//! ```
//!
//! you can then use the struct like this:
//!
//! ```
//! # use default_struct_builder::DefaultBuilder;
//! #
//! # #[derive(DefaultBuilder, Default)]
//! # pub struct SomeOptions {
//! #     throttle: f64,
//! #
//! #     #[builder(into)]
//! #     offset: Option<f64>,
//! #
//! #     #[builder(skip)]
//! #     not_included: u32,
//! # }
//! #
//! # fn main() {
//! let options = SomeOptions::default().offset(4.0);
//!
//! assert_eq!(options.offset, Some(4.0));
//! assert_eq!(options.throttle, 0.0);
//! assert_eq!(options.not_included, 0);
//! # }
//! ```
//!
//! ### Generics
//!
//! The macro is ready to be used on generic structs.
//!
//! ```
//! use default_struct_builder::DefaultBuilder;
//!
//! #[derive(DefaultBuilder, Default)]
//! pub struct SomeOptions<T>
//! where
//!     T: Default,
//! {
//!     some_field: T,
//! }
//! ```
//!
//! ### Doc comments
//!
//! All doc comments on fields are directly passed on to their generated setter methods.
//!
//! ## How it works
//!
//! The derive macro generates the following code:
//!
//! ```
//! # #[derive(Default)]
//! # pub struct SomeOptions {
//! #     throttle: f64,
//! #     offset: Option<f64>,
//! #     not_included: u32,
//! # }
//! #
//! impl SomeOptions {
//!     // setter methods are given that consume `self` and return a new `Self` with the field value changed
//!     pub fn throttle(self, value: f64) -> Self {
//!         Self {
//!             throttle: value,
//!             ..self
//!         }
//!     }
//!
//!     // because `into` was specified this method is generic and calls `.into()` when setting the value
//!     pub fn offset<T>(self, value: T) -> Self
//!     where
//!         T: Into<Option<f64>>,
//!     {
//!         Self {
//!             offset: value.into(),
//!             ..self
//!         }
//!     }
//!
//!     // no method for field `not_included` because `skip` was specified
//! }
//! ```
//!
//! ### Generics
//!
//! In the case of a generic field the generated method is a bit more complex because by calling
//! the method the type of the type parameter can be different than before.
//!
//! Let's look at the following example.
//!
//! ```
//! use default_struct_builder::DefaultBuilder;
//!
//! #[derive(DefaultBuilder, Default)]
//! pub struct SomeOptions<T>
//! where
//!     T: Default,
//! {
//!     some_field: T,
//!     other_field: i16,
//! }
//!
//! impl SomeOptions<f32> {
//!     pub fn new() -> Self {
//!         Self {
//!             some_field: 42.0,
//!             other_field: 0,
//!         }   
//!     }
//! }
//! #
//! # fn main() {
//! #    let options = SomeOptions::new().some_field("string");
//! # }
//! ```
//!
//! This generates the setter method below.
//!
//! ```
//! # pub struct SomeOptions<T>
//! # where
//! #     T: Default,
//! # {
//! #     some_field: T,
//! #     other_field: i16,
//! # }
//! #
//! # impl SomeOptions<f32> {
//! #     pub fn new() -> Self {
//! #         Self {
//! #             some_field: 42.0,
//! #             other_field: 0,
//! #         }   
//! #     }
//! # }
//! #
//! impl<T> SomeOptions<T>
//! where
//!     T: Default,
//! {
//!     pub fn some_field<NewT>(self, value: NewT) -> SomeOptions<NewT>
//!     where
//!         NewT: Default,
//!     {
//!         SomeOptions::<NewT> {
//!             some_field: value,
//!             other_field: self.other_field,
//!         }
//!     }
//! }
//!
//! fn main() {
//!    let options = SomeOptions::new()  // at first    SomeOptions<f32>
//!         .some_field("string");       // changed to  SomeOptions<&str>
//! }
//! ```
//!
//! ## Related Work
//!
//! For more general purposes please check out the much more powerful
//! [`derive_builder` crate](https://github.com/colin-kiegel/rust-derive-builder).

mod builder;

use builder::DefaultBuilderDeriveInput;
use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_derive(DefaultBuilder, attributes(builder))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let data = DefaultBuilderDeriveInput::from_derive_input(&input).expect("Wrong options");
    let stream = quote!(#data);
    stream.into()
}
