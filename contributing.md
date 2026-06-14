# Contributing Guidelines

## Code Style & Standards
### Python typing
- Python typing should be used extensively. Use `basedpyright` to type check your code.
- Typing is mandatory for function/method input arguments. The accepted input type should be as loose as possible (e.g. prefer `Sequence[T]` over `list[T]` if only indexing is required).
- Typing is discouraged for return types, unless it can not be inferred (e.g.: the return type is a supertype such as `list[SomeSuperType]`). If explicit, the hinted return type should be as the inted type should be as accurate as possible (e.g.: prefer `list[T]` over `Sequence[T]`).
- Python bindings are generated with `cargo run --bin stub-gen`.

### Rust unit tests
Unlike the common convention, unit tests live in `src/unit_tests/`, one file per tested module. Each file is wired via a `#[path="..."]` directive by its tested module. Example from `src/solver/context.rs`:
```rust
#[cfg(test)]
#[path = "../unit_tests/test_context.rs"]
mod tests;
```
