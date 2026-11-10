use super::*;

#[test]
fn high_overlap_large_transforms_do_not_hide_unbounded_batch_work() {
    let cfg = StftConfig {
        fft_size: 262144,
        hop: 1,
        window: Window::Hann,
    };
    assert_eq!(ProgressiveOptions::new(4096).unwrap().frames(&cfg, 1), 4);
    assert_eq!(
        ProgressiveOptions::default().frames(&StftConfig::new(2048, Window::Hann), 1),
        1024
    );
}
