impl Shell {
    /// Open UI-only document state without a native reader or FFT pool.
    fn open_test_document(
        &mut self,
        origin: Origin,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (analyst, requests, start) = crate::analysis::readers_for_test();
        let (lease, released) = crate::release::lease();
        let pump = cx.spawn_in(window, async move |_, _| {
            // Retain the genuine mailbox receiver until this document is dropped.
            let _requests = requests;
            std::future::pending::<()>().await;
        });
        self.open_document(
            origin,
            None,
            Some(replace_ui::Readers {
                analyst,
                pump,
                start,
                lease,
                released,
            }),
            window,
            cx,
        );
    }
}
