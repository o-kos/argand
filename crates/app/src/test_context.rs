//! Scope-owned teardown for the application's GPUI tests.

use gpui_kit::TestAppContext;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

#[must_use = "keep this scope until the test's other locals have dropped"]
pub(crate) struct Cleanup(TestAppContext);

/// Declare this before the test's handles so their owners drop before shutdown.
pub(crate) fn cleanup(cx: &TestAppContext) -> Cleanup {
    Cleanup(cx.clone())
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let unwinding = std::thread::panicking();
        let result = catch_unwind(AssertUnwindSafe(|| {
            // The toolkit macro calls App::quit, whose test platform is a no-op.
            self.0.quit();
            self.0.run_until_parked();
        }));
        if let Err(error) = result
            && !unwinding
        {
            resume_unwind(error);
        }
    }
}
