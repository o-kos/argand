use super::replace_ui::tests::{open_window, wait_until, write_wave};
use super::{Origin, Shell};
use gpui_kit::TestAppContext;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

fn document_scope(path: &std::path::Path, unwind: bool) {
    let mut root = None;
    let mut released = None;
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut app = TestAppContext::single();
        let _cleanup = crate::test_context::cleanup(&app);
        let (shell, cx) = open_window(&mut app);
        root = Some(shell.downgrade());
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(path.to_owned()), window, cx);
        });
        wait_until(cx, &shell, "the capture to open", |shell| {
            shell
                .editing()
                .is_some_and(crate::editing::Editing::is_described)
        });
        released = shell.read_with(cx, |shell: &Shell, _| {
            shell.file.as_ref().map(|file| file.released.clone())
        });
        assert!(!unwind, "intentional test-scope unwind");
    }));
    match result {
        Ok(()) => assert!(!unwind, "the intentional panic did not occur"),
        Err(error) if unwind => assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"intentional test-scope unwind"),
            "unexpected failure before the intentional unwind"
        ),
        Err(error) => std::panic::resume_unwind(error),
    }
    assert!(
        root.unwrap().upgrade().is_none(),
        "test window retained its Shell"
    );
    let mut released = std::pin::pin!(released.unwrap().wait());
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let deadline = Instant::now() + Duration::from_secs(5);
    while released.as_mut().poll(&mut context).is_pending() {
        assert!(
            Instant::now() < deadline,
            "test document retained its readers"
        );
        std::thread::park_timeout(Duration::from_millis(1));
    }
}

#[test]
fn context_cleanup_releases_repeated_document_scopes() {
    let directory = argand_io::testutil::TempDir::new("context-cleanup");
    let path = directory.join("capture.wav");
    write_wave(&path, 4096);
    for _ in 0..3 {
        document_scope(&path, false);
    }
}

#[test]
fn context_cleanup_releases_readers_during_unwind() {
    let directory = argand_io::testutil::TempDir::new("context-unwind");
    let path = directory.join("capture.wav");
    write_wave(&path, 4096);
    document_scope(&path, true);
}
