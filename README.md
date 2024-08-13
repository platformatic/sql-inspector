# SQL Inspector

## Prerequisites:

- Rust toolchain: https://www.rust-lang.org/tools/install
- wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
- cargo make: https://github.com/sagiegurari/cargo-make

## Run test

The actual tests are in rust, but there are also some (simple) JS test to test that the JS call works correctly.

To run all of them:

```
cargo make test
```

## Build

```
cargo make wasm
```

The stuff is build in `pkg`

## Publish

```
cd pkg
npm publish
```
