# Default Struct Builder

[![Crates.io](https://img.shields.io/crates/v/default-struct-builder.svg)](https://crates.io/crates/default-struct-builder)
[![Docs](https://docs.rs/default-struct-builder/badge.svg)](https://docs.rs/default-struct-builder/)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/synphonyte/default-struct-builder#license)
[![Build Status](https://github.com/synphonyte/default-struct-builder/actions/workflows/ci.yml/badge.svg)](https://github.com/synphonyte/default-struct-builder/actions/workflows/ci.yml)

<!-- cargo-rdme start -->

Generates builder methods of every field of a struct. It is meant to be used on structs that
implement `Default`. There is no separate builder struct generated and no need to call a
`build()` method at the end or `.unwrap()`.

This crate is used by the crate `leptos-use` for the option structs that
can be passed to the various functions.

### Installation

In your project folder run

```sh
cargo add default-struct-builder
```

### Usage

It is very easy to use:

```rust
use default_struct_builder::DefaultBuilder;

#[derive(DefaultBuilder, Default)]
pub struct SomeOptions {
    throttle: f64,

    #[builder(into)]
    offset: Option<f64>,

    #[builder(skip)]
    not_included: u32,
}
```

you can then use the struct like this:

```rust
let options = SomeOptions::default().offset(4.0);

assert_eq!(options.offset, Some(4.0));
assert_eq!(options.throttle, 0.0);
assert_eq!(options.not_included, 0);
```

### How it works

The derive macro generates the following code:

```rust
impl SomeOptions {
    // setter methods are given that consume `self` and return a new `Self` with the field value changed
    pub fn throttle(self, value: f64) -> Self {
        Self {
            throttle: value,
            ..self
        }
    }

    // because `into` was specified this method is generic and calls `.into()` when setting the value
    pub fn offset<T>(self, value: T) -> Self
    where
        T: Into<Option<f64>>,
    {
        Self {
            offset: value.into(),
            ..self
        }
    }

    // no method for field `not_included` because `skip` was specified
}
```
### Related Work

For more general purposes please check out the much more powerful
[`derive_builder` crate](https://github.com/colin-kiegel/rust-derive-builder).

<!-- cargo-rdme end -->
