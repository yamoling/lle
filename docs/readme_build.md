## Building
This project has been set up using [Poetry](https://python-poetry.org/). To build the project, run the following commands:
```bash
poetry shell
poetry install
maturin develop  # For development
maturin build    # For distribution
```

## Tests
This project **does not** respect Rust unit tests convention and takes inspiration from [this structure](http://xion.io/post/code/rust-unit-test-placement.html). Unit tests are in the `src/unit_tests` folder and are explicitely linked to in each file with the `#path` directive. 
Integration tests are written on the python side.

Run unit tests with 
```bash
cargo test
```

Run integration tests with
```bash
pytest
```