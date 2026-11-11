#[cfg(test)]
impl BadgeEntry {
    pub fn text(&self, cx: &gpui_kit::App) -> String {
        self.input.read(cx).value().to_string()
    }

    pub fn invalid(&self, cx: &gpui_kit::App) -> bool {
        self.marked(cx)
    }
}
