//! Toolkit-independent vertical panel layout.

pub fn waveform_height(total: f32, rem: f32, fraction: Option<f32>, scale: f32) -> f32 {
    let minimum = 3.0 * rem;
    let maximum = (total - minimum).max(minimum);
    let height = fraction
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .map_or(minimum, |value| total * value)
        .clamp(minimum, maximum)
        .min(total.max(0.0));
    (height * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_follows_font_and_adjusted_split_follows_window() {
        assert_eq!(waveform_height(600.0, 16.0, None, 1.0), 48.0);
        assert_eq!(waveform_height(600.0, 20.0, None, 1.0), 60.0);
        assert_eq!(waveform_height(600.0, 16.0, Some(0.25), 1.0), 150.0);
        assert_eq!(waveform_height(800.0, 16.0, Some(0.25), 1.0), 200.0);
    }

    #[test]
    fn small_or_corrupt_layout_stays_bounded() {
        for fraction in [None, Some(f32::NAN), Some(f32::INFINITY), Some(-1.0)] {
            assert_eq!(waveform_height(600.0, 16.0, fraction, 1.0), 48.0);
            assert_eq!(waveform_height(20.0, 16.0, fraction, 1.0), 20.0);
        }
        assert_eq!(waveform_height(600.0, 16.0, Some(1.0), 1.0), 552.0);
    }
}

#[cfg(test)]
#[test]
fn adjusted_split_stays_on_device_pixels_at_fractional_dpi() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let height = waveform_height(316.0, 16.0, Some(0.45253164), scale);
        assert!((height * scale).fract().abs() < 1e-5);
    }
}
