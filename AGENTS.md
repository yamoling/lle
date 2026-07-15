# Agents instructions
## Core Constraints
- **Commits:** You are NOT allowed to create commits on your own initiative, unless explicitly requested by the user.
- **Watermarking:** Every non-trivial function or method you write or edit in the main code of the repository must include an `@ai-generated` tag in its docstring/documentation, unless the edit is minimal or the function is trivial (e.g.: constructors or one-liners). The user will verify and remove this tag later. Files in the `.agents/` folder do not need this waterkmarking.
- **Testing:** When running solver-related tests, always enforce a 60-second timeout to prevent infinite loops. The test structure should be `timeout 60 <test command>; [ $? -eq 124 ] && echo "=== Timeout reached ! ==="`. Examples:
  - `timeout 60 pytest; [ $? -eq 124 ] && echo "=== Timeout reached ! ==="`
  - `timeout 60 cargo test; [ $? -eq 124 ] && echo "=== Timeout reached ! ==="`

## Contextual Imports
@readme.md
@contributing.md

You MUST read the [readme.md](readme.md) and [contributing.md](contributing.md) before writing any code, if you haven't already.

## Project Description
LLE (Laser Learning Environment) is a multi-agent reinforcement learning gridworld implemented as a Rust library with Python bindings via PyO3/maturin. Agents navigate a grid, collect gems, and reach exit tiles while avoiding or blocking laser beams.

### Python Binding Workflow
Each Rust type gets a `Py*` wrapper in `src/bindings/` deriving `#[pyclass]`. Custom PyO3 exceptions reside in `src/bindings/pyexceptions.rs`.
- **Critical Step:** After modifying Rust types exposed to Python, you MUST run `cargo stub-gen` to update the `.pyi` stubs.

### Map Formats
- **Plain-text (v1):** Space-separated tokens per row, newline-separated rows. Explained in `python/lle/__init__.py`.
  - Tokens: `S[id]` (Start), `G` (Gem), `X` (Exit), `.` (Floor), `@` (Wall), `V` (Void), `L[id][direction]` (Laser source: N/E/S/W).
- **TOML (v2):** Richer format supporting random start positions. Automatically detected by the presence of a `[world]` header.
- Built-in levels 1–6 are statically embedded via `build.rs` and `src/core/levels.rs`.

## Behaviour
### Prompt output
At the end of a prompt, you should not explicitly state that you have formatted the code or used the 60 seconds timeout since these are expected from you. Only report useful information related to the prompt or failing tests, if applicable.

### Testing
For Rust tests, you should use the short output format with `cargo test -- --format terse`, unless you are debugging a specific test case.

## Watermark Examples

### Rust
```rust
struct MyStruct {
    a: usize,
}

impl MyStruct {
    /// Trivial constructor: No tag required.
    pub fn new(a: usize) -> Self {
        Self { a }
    }

    /// Complex function example.
    /// 
    /// @ai-generated
    fn complex_algorithm(&self) -> bool {
        self.a > 42
    }
}

#[cfg(test)]
mod tests {
    /// Tests must also have the @ai-generated tag.
    /// 
    /// @ai-generated
    #[test]
    fn my_struct_constructor() {
        let s = MyStruct::new(10);
        assert_eq!(s.a, 10);
    }
}
```

### Python
```python
def complex_function():
    """
    Executes core agent logic.
    
    @ai-generated
    """
    return True
```
