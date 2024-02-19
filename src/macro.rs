#[macro_export]
macro_rules! database_call {
    ($function:expr) => {{
        $crate::CException::handle(
            "unknown database action",
            core::panic::AssertUnwindSafe(|| unsafe { $function }),
        )
    }};
    ($action:expr, $function:expr) => {{
        // tracing::trace!("{} at line {}", stringify!($function), line!());
        tracing::trace!(
            target: "database", // ekg_namespace::consts::LOG_TARGET_DATABASE,
            "{}",
            $action
        );
        rdfox_sys::CException::handle(
            $action,
            core::panic::AssertUnwindSafe(|| unsafe { $function }),
        )
    }};
}
