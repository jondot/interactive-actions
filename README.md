Interactive Actions
===================

[<img alt="github" src="https://img.shields.io/badge/github-jondot/interactive--actions-8dagcb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/jondot/interactive-actions)
[<img alt="crates.io" src="https://img.shields.io/crates/v/interactive-actions.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/interactive-actions)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-interactive_actions-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/interactive-actions)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/jondot/interactive-actions/Build/master?style=for-the-badge" height="20">](https://github.com/jondot/interactive-actions/actions?query=branch%3Amaster)

This is a Rust library that runs actions and interactions that are defined declaratively.

## Dependency

```toml
[dependencies]
interactive-actions = "1"
```

For most recent version see [crates.io](https://crates.io/crates/interactive-actions)


## Usage

Run the example:

```rust
$ cargo run --example interaction
Finished dev [unoptimized + debuginfo] target(s) in 0.30s
   Running `target/debug/examples/interaction`
start
✔ are you ready to start? · Yes
city
✔ input a city · goo
transport
✔ pick a transport · bus
+ cd projects/interactive-actions
+ echo go for goo on a bus
go for goo on a bus
```

Create a `Vec<Action>`, here described in YAML, but you can either build it in Rust or any `Deserialize` format:

```yaml
- name: city
  interaction:
    kind: input
    prompt: input a city
    out: city
```

And run it:

```rust
use interactive_actions::{data::{Action, ActionHook}, ActionRunner};

let actions: Vec<Action> = serde_yaml::from_str(YAML).unwrap();
let mut runner = ActionRunner::default();
let res = runner.run(
   &actions,
   None,
   ActionHook::After,
   Some(|action: &Action| {
      println!("{}", action.name);
   }),
);
```






# Copyright

Copyright (c) 2022 [@jondot](http://twitter.com/jondot). See [LICENSE](LICENSE.txt) for further details.
