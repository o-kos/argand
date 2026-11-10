use super::*;

#[test]
fn batches_reject_invalid_sizes_and_bound_decoded_memory() {
    assert!(ProgressiveOptions::new(0).is_err());
    assert!(ProgressiveOptions::new(4097).is_err());
    let options = ProgressiveOptions::new(4096).unwrap();
    for channels in [1, 2] {
        for size in [2048, 65536, 1 << 22] {
            let cfg = StftConfig::new(size, Window::Hann);
            let frames = options.frames(&cfg, channels);
            let values = (cfg.fft_size + (frames - 1) * cfg.hop) * channels;
            assert!(values <= (1 << 20) || frames == 1);
        }
    }
}
