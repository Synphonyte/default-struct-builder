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

#### Generics

The macro is ready to be used on generic structs.

```rust
use default_struct_builder::DefaultBuilder;

#[derive(DefaultBuilder, Default)]
pub struct SomeOptions<T>
where
    T: Default,
{
    some_field: T,
}
```

#### Doc comments

All doc comments on fields are directly passed on to their generated setter methods.

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

#### Generics

In the case of a generic field the generated method is a bit more complex because by calling
the method the type of the type parameter can be different than before.

Let's look at the following example.

```rust
use default_struct_builder::DefaultBuilder;

#[derive(DefaultBuilder, Default)]
pub struct SomeOptions<T>
where
    T: Default,
{
    some_field: T,
    other_field: i16,
}

impl SomeOptions<f32> {
    pub fn new() -> Self {
        Self {
            some_field: 42.0,
            other_field: 0,
        }   
    }
}
```

This generates the setter method below.

```rust
impl<T> SomeOptions<T>
where
    T: Default,
{
    pub fn some_field<NewT>(self, value: NewT) -> SomeOptions<NewT>
    where
        NewT: Default,
    {
        SomeOptions::<NewT> {
            some_field: value,
            other_field: self.other_field,
        }
    }
}

fn main() {
   let options = SomeOptions::new()  // at first    SomeOptions<f32>
        .some_field("string");       // changed to  SomeOptions<&str>
}
```

In cases where you don't want a generic field to be able to change the generic type you
can annotate it with `keep_type`.

```rust
#[derive(DefaultBuilder)]
struct SomeOptions<T> {
    #[builder(keep_type)]
    the_field: T,
}
```

this will generate a standard builder method as if `T` wasn't generic.

#### `Box`, `Rc` and `Arc`

The macro detects if a field is a `Box` (or `Rc` or `Arc`) and generates a builder method that
accepts the inner type (without `Box` or `Rc` or `Arc`) and adds the outer type in the body.

In case it's a `Box<dyn Trait>` the builder method will have an argument of type
`impl Trait`. The same goes for `Rc` and `Arc`.

If you want to prevent this auto un-wrapping you can use the `#[builder(keep_outer)]` attribute.

```rust
trait Test {}

#[derive(DefaultBuilder)]
struct SomeOptions {
    the_field: Box<dyn Test>,
    other_field: Rc<String>,

    #[builder(keep_outer)]
    keep: Box<String>,
}
```

This will generate the following code:

```rust
impl SomeOptions {
    pub fn the_field(self, value: impl Test + 'static) -> Self {
        Self {
            the_field: Box::new(value),
            ..self
        }   
    }

    pub fn other_field(self, value: String) -> Self {
        Self {
            other_field: Rc::new(value),
            ..self
        }
    }

    pub fn keep(self, value: Box<String>) -> Self {
        Self {
            keep: value,
            ..self
        }   
    }
}
```


### Related Work

For more general purposes please check out the much more powerful
[`derive_builder` crate](https://github.com/colin-kiegel/rust-derive-builder).

<!-- cargo-rdme end -->
