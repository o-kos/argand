//! Toolkit-independent vertical panel layout.

/// The minimap's size across the panel, 3 rem on device pixels, never more than the panel holds.
pub fn waveform_height(total: f32, rem: f32, scale: f32) -> f32 {
    let height = (3.0 * rem).min(total.max(0.0));
    (height * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_minimap_follows_the_font_and_not_the_window() {
        assert_eq!(waveform_height(600.0, 16.0, 1.0), 48.0);
        assert_eq!(waveform_height(800.0, 16.0, 1.0), 48.0);
        assert_eq!(waveform_height(600.0, 20.0, 1.0), 60.0);
    }

    #[test]
    fn a_small_panel_bounds_the_minimap() {
        assert_eq!(waveform_height(20.0, 16.0, 1.0), 20.0);
        assert_eq!(waveform_height(-5.0, 16.0, 1.0), 0.0);
    }

    #[test]
    fn the_minimap_stays_on_device_pixels_at_fractional_dpi() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let height = waveform_height(316.0, 15.3, scale);
            assert!((height * scale).fract().abs() < 1e-5);
        }
    }
}
