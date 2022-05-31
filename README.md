# rt-local

[![Crates.io](https://img.shields.io/crates/v/rt-local.svg)](https://crates.io/crates/rt-local)
[![Docs.rs](https://docs.rs/rt-local/)](https://docs.rs/rt-local/badge.svg)
[![Actions Status](https://github.com/frozenlib/rt-local/workflows/CI/badge.svg)](https://github.com/frozenlib/rt-local/actions)

Thread local async runtime.

## Example

```rust
#[rt_local::runtime::core::main]
async fn main() {
  // ...
}
```

```rust
fn main() {
  rt_local::runtime::core::run(async {
    // ...
  });
}
```

## License

This project is dual licensed under Apache-2.0/MIT. See the two LICENSE-\* files for details.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
