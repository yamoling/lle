use std::sync::Once;
static INIT: Once = Once::new();
#[macro_export]
macro_rules! log_info {
        ($($arg:tt)*) => {
            log::info!(
                target: module_path!(),
                "[{}:{}] - INFO - {}",
                file!(),
                line!(),
                format!($($arg)*)
            )
        };
    }

#[macro_export]
macro_rules! log_warn {
        ($($arg:tt)*) => {
            log::warn!(
                target: module_path!(),
                "[{}:{}] - WARN - {}",
                file!(),
                line!(),
                format!($($arg)*)
            )
        };
    }

#[macro_export]
macro_rules! log_error {
        ($($arg:tt)*) => {
            log::error!(
                target: module_path!(),
                "[{}:{}] - ERROR - {}",
                file!(),
                line!(),
                format!($($arg)*)
            )
        };
    }

#[macro_export]
macro_rules! log_debug {
        ($($arg:tt)*) => {
            log::debug!(
                target: module_path!(),
                "[{}:{}] - DEBUG - {}",
                file!(),
                line!(),
                format!($($arg)*)
            )
        };
    }
pub fn init_pylogging() {
    INIT.call_once(|| {
        pyo3_log::init();
    });
}
