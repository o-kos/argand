#[cfg(test)]
#[test]
fn incremental_overview_render_matches_a_fresh_render_with_empty_followers() {
    use argand_core::{Domain, SampleFormat, SampleType};
    for reduce in [Reduce::Max, Reduce::MeanPower] {
        let meta = SignalMeta {
            sample_rate: 1000.0,
            center_freq: 0.0,
            sample_type: SampleType::new(Domain::Iq, SampleFormat::F32),
            len_samples: 1000,
            container: "test",
            divisor: 1.0,
            source: "memory".into(),
        };
        let request = AnalysisRequest {
            cfg: StftConfig::new(8, Window::Hann),
            range: SampleRange::new(0, 1000),
            width: 7,
            height: 4,
            reduce,
            colormap: Colormap::Oceanic,
            dynamic_range: DynamicRange::Auto,
            waveform_columns: None,
        };
        let mut state = Overview::with_cache(meta, request, false).unwrap();
        for (column, value) in [(0, 0.1), (6, 0.8), (0, 0.6), (3, 0.2), (3, 0.7)] {
            let mut partial = state.partial();
            partial.cols = vec![column];
            partial.rows = vec![value, value * 0.5, value * 0.1, 0.0];
            partial.frames = 1;
            state.absorb(&partial);
            let incremental = state.render(5, 3, None).unwrap();
            state.view_cache = None;
            let fresh = state.render(5, 3, None).unwrap();
            assert_eq!(incremental.db.values, fresh.db.values);
            assert_eq!(incremental.spectrogram.rgba, fresh.spectrogram.rgba);
        }
        state.display_scale = None;
        let final_view = state.render(5, 3, None).unwrap();
        state.view_cache = None;
        let fresh = state.render(5, 3, None).unwrap();
        assert_eq!(final_view.db.values, fresh.db.values);
        assert_eq!(final_view.spectrogram.rgba, fresh.spectrogram.rgba);
    }
}
