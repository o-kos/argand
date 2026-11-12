//! Fixed-parameter, window-free progressive benchmark: PATH [WORKERS] [BATCH].
use std::{path::Path, time::Instant};

use argand_core::{Colormap, SampleRange};
use argand_dsp::{
    AnalysisRequest, DynamicRange, Flow, ProgressiveOptions, Reduce, StftConfig, Window,
    analyze_progressive_with_options,
};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("expected PATH [WORKERS] [BATCH]"))?;
    let workers: usize = args.next().unwrap_or_else(|| "8".into()).parse()?;
    let batch: usize = args.next().unwrap_or_else(|| "1024".into()).parse()?;
    anyhow::ensure!(
        (1..=128).contains(&workers),
        "workers must be from 1 to 128"
    );
    let options = ProgressiveOptions::new(batch)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()?;
    let hints = argand_io::OpenHints {
        level_scan_bytes: Some(64 << 20),
        ..Default::default()
    };
    let opened = Instant::now();
    let mut source = argand_io::open(Path::new(&path), &hints)?;
    let opening = opened.elapsed().as_secs_f64();
    let request = AnalysisRequest {
        cfg: StftConfig::new(2048, Window::Hann),
        range: SampleRange::new(0, source.meta().len_samples),
        width: 1214,
        height: 662,
        reduce: Reduce::Max,
        colormap: Colormap::Oceanic,
        dynamic_range: DynamicRange::Default,
        waveform_columns: Some(1214),
    };
    let started = Instant::now();
    let mut first = None;
    let mut updates = 0;
    let result = pool.install(|| {
        analyze_progressive_with_options(
            source.as_mut(),
            &request,
            options,
            &|| Flow::Continue,
            &mut |_, _| {
                first.get_or_insert(started.elapsed().as_secs_f64());
                updates += 1;
                Flow::Continue
            },
        )
    })?;
    let elapsed = started.elapsed().as_secs_f64();
    let mut hash = 0xcbf29ce484222325u64;
    for value in &result.db.values {
        for byte in value.to_bits().to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    println!("workers,batch,opening_seconds,analysis_seconds,first_seconds,updates,frames,db_hash");
    println!(
        "{workers},{batch},{opening:.6},{elapsed:.6},{:.6},{updates},{},{hash:016x}",
        first.unwrap_or(elapsed),
        result.frames
    );
    Ok(())
}
