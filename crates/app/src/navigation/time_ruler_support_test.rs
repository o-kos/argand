impl Ruler {
    #[cfg(test)]
    pub const CLOCK: Self = Self {
        mode: Mode::Clock,
        view: View { start: 0, len: 1 },
        total: 1,
    };
}
