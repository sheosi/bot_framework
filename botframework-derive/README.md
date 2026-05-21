# botframework-derive

Procedural macro crate for `botframework`.

## Derive Macros

### `#[derive(MyTrait)]`

Implements the `MyTrait` trait for your type.

```rust
use botframework::{MyTrait, MyTrait};

#[derive(MyTrait)]
struct MyStruct;

let s = MyStruct;
assert_eq!(s.my_trait_method(), "MyStruct");
```

### `#[derive(Builder)]`

Generates a builder pattern for your struct.

```rust
use botframework::Builder;

#[derive(Builder)]
struct Config {
    name: String,
    value: i32,
}

let config = Config::builder()
    .name("test".to_string())
    .value(42)
    .build()
    .unwrap();
```
