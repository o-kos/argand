impl Frame {
    /// Reserve room for the labels, then lay out the marks in what is left.
    ///
    /// `scale` is the display's, and the plot's edges are snapped to whole
    /// device pixels with it. That is what lets a transform asked for exactly
    /// this many columns be drawn one column to one pixel: a plot half a
    /// device pixel wide at one edge is resampled by the GPU, and a resampled
    /// spectrogram is a blurred one.
    ///
    /// `None` when the panel is too small to hold a plot at all, which is the
    /// answer for a window dragged down to nothing: there is no rectangle to
    /// draw into and nothing to ask a transform for.
    #[cfg(test)]
    pub fn measure(
        panel: Size<Pixels>,
        scale: f32,
        extents: Extents,
        measure: &dyn LabelMeasure,
        held: Option<axis::TickScheme>,
    ) -> Option<Self> {
        let held = Held {
            time: held,
            ..Held::default()
        };
        Self::measure_view(panel, scale, extents, measure, held)
    }
}
