# Contributing Guidelines

Welcome to the LLE contributing guidelines.

## Code Style & Standards
### Language
Everything should be documented in English with [Oxford spelling](https://en.wikipedia.org/wiki/Oxford_spelling). 

From Wikipedia: "Oxford spelling uses the spelling ‑ize alongside ‑lyse: organization, realize, privatize and recognizable, rather than organisation, realise, privatise and recognisable – but analyse and paralyse. Words such as advise, advertise, improvise, surprise are spelled thus in all varieties of English, since ‑ise in them is not a suffix, but a part of an English or French root."

Additionally, words in "-our" such as "colour", "neighbour" or "behaviour" should be spelled with an "u" (e.g. "colour" not "color").

### Python typing
- Python typing should be used extensively. Use `basedpyright` to type check your code.
- Typing is mandatory for function/method input arguments. The accepted input type should be as loose as possible (e.g. prefer `Sequence[T]` over `list[T]` if only indexing is required).
- Typing is discouraged for return types, unless it can not be inferred (e.g.: the return type is a supertype such as `list[SomeSuperType]`). If explicit, the hinted return type should be as the inted type should be as accurate as possible (e.g.: prefer `list[T]` over `Sequence[T]`).
- Python bindings are generated with `cargo run --features python-bindings --bin stub-gen`.

### Rust unit tests
Unlike the common convention, unit tests live in `src/unit_tests/`, one file per tested module. Each file is wired via a `#[path="..."]` directive by its tested module. Example from `src/solver/context.rs`:
```rust
#[cfg(test)]
#[path = "../unit_tests/test_context.rs"]
mod tests;
```

### Documentation
Function documentation should be written in markdown format. Examples are appreciated if they help understand the function's behaviour.
