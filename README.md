# rt-local

[![Crates.io](https://img.shields.io/crates/v/rt-local.svg)](https://crates.io/crates/rt-local)
[![Docs.rs](https://docs.rs/rt-local/badge.svg)](https://docs.rs/rt-local/)
[![Actions Status](https://github.com/frozenlib/rt-local/workflows/CI/badge.svg)](https://github.com/frozenlib/rt-local/actions)

Thread local asynchronous runtime working with platform-specific event loops.

## Example

```rust
use rt_local::spawn_local;
use rt_local::runtime::core::main;

#[main]
async fn main() {
  let task_a = spawn_local(async {
    // ...
  });
  let task_b = spawn_local(async {
    // ...
  });
  task_a.await;
  task_b.await;
}
```

## Features

| crate feature | module                      | backend              |
| ------------- | --------------------------- | -------------------- |
|               | [`core`][module_core]       | platform independent |
| `windows`     | [`windows`][module_windows] | windows message loop |

[module_core]: https://docs.rs/rt-local/latest/rt_local/runtime/core/
[module_windows]: https://docs.rs/rt-local/latest/rt_local/runtime/windows/

## License

This project is dual licensed under Apache-2.0/MIT. See the two LICENSE-\* files for details.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
