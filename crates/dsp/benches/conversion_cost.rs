use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use argand_dsp::convert::ConversionPlan;

fn meta(iq: bool, rate: f64, reference: f64) -> SignalMeta {
    SignalMeta {
        sample_rate: rate,
        center_freq: reference,
        sample_type: SampleType::new(
            if iq { Domain::Iq } else { Domain::Real },
            SampleFormat::F32,
        ),
        len_samples: 16_384,
        container: "wav",
        divisor: 1.0,
        source: "tone.wav".into(),
    }
}

fn main() {
    for (iq, output_rate, reference) in [
        (true, 50_000.0, 0.0),
        (true, 1_000_000.0, 100_000.0),
        (false, 1_000_000.0, 250_000.0),
    ] {
        let mut input = meta(iq, 1_000_000.0, 0.0);
        input.len_samples = 1_000_000;
        let output = meta(true, output_rate, reference);
        let started = std::time::Instant::now();
        let plan = ConversionPlan::new(&input, &output).unwrap();
        let mut stream = plan.stream(input.len_samples).unwrap();
        let prepared = started.elapsed();
        let block = vec![0.125; 4096 * input.channels()];
        let mut count = 0u64;
        let mut sink = |values: &[[f32; 2]]| -> Result<(), ()> {
            count += values.len() as u64;
            Ok(())
        };
        let mut remaining = input.len_samples as usize;
        while remaining > 0 {
            let take = remaining.min(4096);
            stream
                .push(
                    &block[..take * input.channels()],
                    input.channels(),
                    &mut sink,
                )
                .unwrap();
            remaining -= take;
        }
        stream.finish(&mut sink).unwrap();
        assert_eq!(count, plan.samples);
        eprintln!(
            "input IQ={iq}, output {output_rate} Hz ref={reference}: prepare={prepared:?}, total={:?}, frames={count}",
            started.elapsed()
        );
    }
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status
            .lines()
            .filter(|line| line.starts_with("VmHWM:") || line.starts_with("VmRSS:"))
        {
            eprintln!("{line}");
        }
    }
}
