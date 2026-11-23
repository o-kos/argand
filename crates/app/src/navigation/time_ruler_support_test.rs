impl Ruler {
    #[cfg(test)]
    pub const CLOCK: Self = Self {
        start: None,
        mode: Mode::Clock,
        view: View { start: 0, len: 1 },
        total: 1,
    };
}
