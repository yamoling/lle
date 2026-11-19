//! Tests of the conversion of Rust errors into Python exceptions.
//!
//! These tests need the `python-bindings` feature: `cargo test --features python-bindings`.
use pyo3::{
    PyErr, PyTypeInfo, Python,
    exceptions::{PyFileNotFoundError, PyValueError},
    types::PyAnyMethods,
};

use super::{
    InvalidActionError, InvalidLevelError, InvalidWorldStateError, ParsingError,
    UnsupportedFeatureError, continuous_error_to_pyexception,
};
use crate::{ParseError, continuous::ContinuousError};

/// Assert that `error` is exactly an instance of `T` (not of a subclass), and return its
/// message.
///
/// @ai-generated
fn assert_exact_type<T: PyTypeInfo>(py: Python<'_>, error: &PyErr) -> String {
    let expected = py.get_type::<T>();
    assert!(
        error.get_type(py).is(&expected),
        "expected {expected}, got {}",
        error.get_type(py)
    );
    error.value(py).to_string()
}

#[test]
fn continuous_errors_map_to_the_documented_python_exceptions() {
    Python::initialize();
    Python::attach(|py| {
        let error = continuous_error_to_pyexception(ContinuousError::UnsupportedFeature {
            features: vec!["boxes".into(), "lifts".into()],
        });
        let message = assert_exact_type::<UnsupportedFeatureError>(py, &error);
        assert_eq!(message, "Unsupported features: boxes, lifts");
        assert!(error.is_instance_of::<ParsingError>(py));
        assert!(error.is_instance_of::<PyValueError>(py));

        let error = continuous_error_to_pyexception(ContinuousError::InvalidParams {
            reason: "dt must be positive".into(),
        });
        let message = assert_exact_type::<PyValueError>(py, &error);
        assert_eq!(message, "dt must be positive");

        let error = continuous_error_to_pyexception(ContinuousError::InvalidNumberOfActions {
            given: 1,
            expected: 2,
        });
        let message = assert_exact_type::<PyValueError>(py, &error);
        assert!(
            message.contains("given 1") && message.contains("expected 2"),
            "{message}"
        );

        let error = continuous_error_to_pyexception(ContinuousError::InvalidAction {
            agent_id: 1,
            action: [f64::NAN, 0.0],
        });
        let message = assert_exact_type::<InvalidActionError>(py, &error);
        assert!(message.contains("agent 1"), "{message}");

        let error = continuous_error_to_pyexception(ContinuousError::InvalidState {
            reason: "Agents overlap".into(),
        });
        let message = assert_exact_type::<InvalidWorldStateError>(py, &error);
        assert_eq!(message, "Agents overlap");
    });
}

#[test]
fn continuous_parse_errors_are_converted_like_parse_errors() {
    Python::initialize();
    Python::attach(|py| {
        let error =
            continuous_error_to_pyexception(ContinuousError::Parse(ParseError::InvalidLevel {
                asked: 7,
                min: 1,
                max: 6,
            }));
        let message = assert_exact_type::<InvalidLevelError>(py, &error);
        assert!(message.contains('7'), "{message}");

        let error =
            continuous_error_to_pyexception(ContinuousError::Parse(ParseError::InvalidFileName {
                file_name: "missing.toml".into(),
            }));
        assert_exact_type::<PyFileNotFoundError>(py, &error);

        let error = continuous_error_to_pyexception(ContinuousError::Parse(ParseError::NoAgents));
        let message = assert_exact_type::<ParsingError>(py, &error);
        assert_eq!(message, "No agents in the world");
    });
}
