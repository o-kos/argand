//! Measure the image-sized copy and shading work of a progressive snapshot.

use std::hint::black_box;
use std::time::Instant;

use argand_core::{Colormap, DbGrid};
use argand_dsp::{Shading, shade};

fn main() {
    let width = 1600;
    let height = 600;
    let grid = DbGrid {
        width,
        height,
        values: (0..width * height)
            .map(|i| -110.0 + (i % 1100) as f32 * 0.1)
            .collect(),
        t0: 0.0,
        t1: 100.0,
        f0: -1_000_000.0,
        f1: 1_000_000.0,
    };
    let shading = Shading {
        colormap: Colormap::Oceanic,
        db_min: -110.0,
        db_max: 0.0,
    };
    black_box(shade(&grid, shading));
    let started = Instant::now();
    for _ in 0..100 {
        let snapshot = grid.clone();
        black_box(shade(&snapshot, shading));
    }
    let ms = started.elapsed().as_secs_f64() * 10.0;
    println!(
        "{width}x{height}: grid copy + shade {ms:.3} ms/snapshot; {:.1} ms CPU/s at 20 Hz; {} RGBA bytes/snapshot",
        ms * 20.0,
        width * height * 4
    );
    println!("Excludes FFT, level resolution, waveform cloning, GUI upload and presentation");
}
