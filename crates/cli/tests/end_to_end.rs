//! Whole-binary tests: fixtures on disk, `aspec` invoked as a user would.
//!
//! The fixtures are generated rather than committed, which is what makes it
//! possible to cover all ten sample types -- including `u8` and `i32`, which
//! no real capture to hand happens to use.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use argand_io::testutil::{TempDir, all_sample_types, iq_tone, real_tone, write_raw, write_wav};
use serde_json::Value;

const ASPEC: &str = env!("CARGO_BIN_EXE_aspec");
const RATE: u32 = 24_000;
const FFT: usize = 256;
/// Exactly on a bin, so quantisation cannot nudge the peak next door.
const TONE_BIN: usize = 40;
const TONE_HZ: f64 = TONE_BIN as f64 * RATE as f64 / FFT as f64;
const SAMPLES: usize = 8192;

fn run(args: &[&str]) -> Output {
    Command::new(ASPEC)
        .args(args)
        .output()
        .expect("aspec should be runnable")
}

fn run_json(args: &[&str]) -> Value {
    let out = run(&[args, &["--json", "--quiet"]].concat());
    assert!(
        out.status.success(),
        "aspec {args:?} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "invalid json from aspec {args:?}: {e}\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

fn png(dir: &TempDir, name: &str) -> PathBuf {
    dir.join(name)
}

fn assert_png(path: &Path) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(
        bytes.len() > 1000,
        "{} is suspiciously small",
        path.display()
    );
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
}

fn fft_args() -> Vec<&'static str> {
    vec!["-f", "256", "--ref", "peak", "-d", "60", "-i", "500x300"]
}

#[test]
fn every_sample_type_puts_the_tone_in_the_same_place() {
    let dir = TempDir::new("e2e-matrix");

    for sample_type in all_sample_types() {
        let values = if sample_type.is_iq() {
            iq_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8)
        } else {
            real_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8)
        };
        // The unnormalised format is written far off unit scale, as a real
        // capture from that chain would be.
        let scale = if sample_type.to_string().ends_with("f16x8") {
            4000.0
        } else {
            1.0
        };
        let input = write_wav(
            &dir.join(&format!("{sample_type}.wav")),
            sample_type,
            RATE,
            &values,
            scale,
        );
        let output = png(&dir, &format!("{sample_type}.png"));

        let report = run_json(
            &[
                &[input.to_str().unwrap(), "-o", output.to_str().unwrap()],
                fft_args().as_slice(),
            ]
            .concat(),
        );

        assert_eq!(
            report["sample_type"],
            sample_type.to_string(),
            "type detection for {sample_type}"
        );
        assert_eq!(report["container"], "wav");
        assert_eq!(report["sample_rate"], 24_000.0);

        let expected_bin = if sample_type.is_iq() {
            FFT / 2 + TONE_BIN
        } else {
            TONE_BIN
        };
        assert_eq!(
            report["peak_bin"]["bin"].as_u64().unwrap() as usize,
            expected_bin,
            "{sample_type} put the tone in the wrong bin"
        );
        let freq = report["peak_bin"]["freq_hz"].as_f64().unwrap();
        assert!(
            (freq.abs() - TONE_HZ).abs() < 1.0,
            "{sample_type}: {freq} Hz, expected {TONE_HZ}"
        );
        assert_png(&output);
    }
}

#[test]
fn a_headerless_file_matches_the_same_signal_in_a_wav() {
    let dir = TempDir::new("e2e-raw");
    let values = iq_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let sample_type = "iq_i16".parse().unwrap();

    let wav = write_wav(&dir.join("ref.wav"), sample_type, RATE, &values, 1.0);
    let raw = write_raw(
        &dir.join("dump.bin"),
        argand_core::SampleFormat::I16,
        &values,
        1.0,
    );

    let from_wav = run_json(
        &[
            &[
                wav.to_str().unwrap(),
                "-o",
                png(&dir, "w.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    let from_raw = run_json(
        &[
            &[
                raw.to_str().unwrap(),
                "--raw",
                "iq_i16@24k",
                "-o",
                png(&dir, "r.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );

    assert_eq!(from_raw["container"], "raw");
    assert_eq!(from_wav["peak_bin"]["bin"], from_raw["peak_bin"]["bin"]);
    assert_eq!(from_wav["samples"], from_raw["samples"]);
    let (a, b) = (
        from_wav["peak_bin"]["dbfs"].as_f64().unwrap(),
        from_raw["peak_bin"]["dbfs"].as_f64().unwrap(),
    );
    assert!((a - b).abs() < 1e-6, "{a} vs {b}");
}

#[test]
fn auto_normalization_rescues_an_unnormalised_float_capture() {
    let dir = TempDir::new("e2e-norm");
    let sample_type = "iq_f16x8".parse().unwrap();
    let values = iq_tone(SAMPLES, RATE as f64, TONE_HZ, 1.0);
    let input = write_wav(&dir.join("hot.wav"), sample_type, RATE, &values, 4000.0);

    // The default for this format measures the file and divides by the peak.
    let auto = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-o",
                png(&dir, "auto.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    assert_eq!(auto["normalize"], "auto");
    assert!(auto["divisor"].as_f64().unwrap() > 4000.0);
    let peak = auto["peak_bin"]["dbfs"].as_f64().unwrap();
    assert!((-1.0..0.0).contains(&peak), "peak came out at {peak} dBFS");

    // Turning it off leaves the raw scale, which reads far above full scale.
    let raw = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-n",
                "none",
                "-o",
                png(&dir, "none.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    assert_eq!(raw["normalize"], "none");
    assert_eq!(raw["divisor"], 1.0);
    assert!(raw["peak_bin"]["dbfs"].as_f64().unwrap() > 60.0);
}

#[test]
fn gain_moves_the_level_and_nothing_else() {
    let dir = TempDir::new("e2e-gain");
    let sample_type = "rl_i16".parse().unwrap();
    let values = real_tone(SAMPLES, RATE as f64, TONE_HZ, 0.25);
    let input = write_wav(&dir.join("quiet.wav"), sample_type, RATE, &values, 1.0);

    let plain = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-o",
                png(&dir, "p.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    let boosted = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-g",
                "6.0206",
                "-o",
                png(&dir, "b.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );

    assert_eq!(plain["peak_bin"]["bin"], boosted["peak_bin"]["bin"]);
    let lift =
        boosted["peak_bin"]["dbfs"].as_f64().unwrap() - plain["peak_bin"]["dbfs"].as_f64().unwrap();
    assert!((lift - 6.02).abs() < 0.05, "+6 dB of gain gave {lift} dB");
}

#[test]
fn negative_gain_parses_as_a_value_not_a_flag() {
    let dir = TempDir::new("e2e-negative");
    let sample_type = "rl_i16".parse().unwrap();
    let values = real_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("x.wav"), sample_type, RATE, &values, 1.0);

    let report = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-g",
                "-6.0206",
                "--center",
                "-1M",
                "-o",
                png(&dir, "n.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    assert_eq!(report["gain_db"], -6.0206);
    assert_eq!(report["center_freq"], -1_000_000.0);
}

#[test]
fn a_span_can_be_selected_out_of_a_longer_capture() {
    let dir = TempDir::new("e2e-span");
    let sample_type = "iq_i16".parse().unwrap();
    let values = iq_tone(RATE as usize * 4, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("long.wav"), sample_type, RATE, &values, 1.0);

    let whole = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-o",
                png(&dir, "whole.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    assert_eq!(whole["duration_seconds"], 4.0);
    assert_eq!(whole["analysed_seconds"], 4.0);

    let part = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "--start",
                "1",
                "--duration",
                "2",
                "-o",
                png(&dir, "part.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    assert_eq!(part["duration_seconds"], 4.0, "the file is still 4 s long");
    assert_eq!(part["analysed_seconds"], 2.0);
    assert!(part["stft"]["frames"].as_u64().unwrap() < whole["stft"]["frames"].as_u64().unwrap());
    assert_eq!(part["peak_bin"]["bin"], whole["peak_bin"]["bin"]);
}

#[test]
fn every_panel_set_and_orientation_produces_a_png() {
    let dir = TempDir::new("e2e-layout");
    let sample_type = "iq_i16".parse().unwrap();
    let values = iq_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("x.wav"), sample_type, RATE, &values, 1.0);

    let mut sizes = Vec::new();
    for panels in ["none", "waveform", "psd", "waveform,psd,db"] {
        for orientation in ["horizontal", "vertical"] {
            let name = panels.replace(',', "-");
            let output = png(&dir, &format!("{name}-{orientation}.png"));
            let out = run(&[
                input.to_str().unwrap(),
                "--panels",
                panels,
                "--orientation",
                orientation,
                "-f",
                "256",
                "-i",
                "480x360",
                "-o",
                output.to_str().unwrap(),
                "--quiet",
            ]);
            assert!(
                out.status.success(),
                "{panels}/{orientation}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_png(&output);
            sizes.push(std::fs::metadata(&output).unwrap().len());
        }
    }
    assert!(
        sizes.iter().collect::<std::collections::HashSet<_>>().len() > 1,
        "the layouts should not all render identically"
    );
}

#[test]
fn the_default_render_is_a_waveform_and_a_spectrogram() {
    let dir = TempDir::new("e2e-default-panels");
    let sample_type = "iq_i16".parse().unwrap();
    let values = iq_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("x.wav"), sample_type, RATE, &values, 1.0);

    let report = run_json(
        &[
            &[
                input.to_str().unwrap(),
                "-o",
                png(&dir, "default.png").to_str().unwrap(),
            ],
            fft_args().as_slice(),
        ]
        .concat(),
    );
    assert_eq!(report["output"]["panels"], "waveform");

    // The spectrum panel is opt-in, but the report still measures one: the
    // transform runs whatever is drawn.
    assert!(report["peak_bin"]["bin"].is_number());
    assert!(report["stft"]["frames"].as_u64().unwrap() > 0);
}

#[test]
fn an_unusable_panel_list_is_refused_with_an_explanation() {
    let dir = TempDir::new("e2e-bad-panels");
    let sample_type = "rl_i16".parse().unwrap();
    let values = real_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("x.wav"), sample_type, RATE, &values, 1.0);

    for (panels, expected) in [
        // The spectrogram is not a panel: it is always drawn.
        ("spectrogram", "waveform, psd, db, none"),
        ("waterfall", "unknown panel"),
        ("", "use `none`"),
        ("none,db", "cannot be combined"),
    ] {
        let out = run(&[input.to_str().unwrap(), "--panels", panels, "--quiet"]);
        assert!(!out.status.success(), "--panels {panels} was accepted");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(expected),
            "--panels {panels} said: {stderr}"
        );
    }
}

#[test]
fn the_report_reaches_stderr_and_the_path_reaches_stdout() {
    let dir = TempDir::new("e2e-streams");
    let sample_type = "rl_i16".parse().unwrap();
    let values = real_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("x.wav"), sample_type, RATE, &values, 1.0);
    let output = png(&dir, "out.png");

    let out = run(&[
        input.to_str().unwrap(),
        "-f",
        "256",
        "-o",
        output.to_str().unwrap(),
    ]);
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stdout.trim(),
        output.display().to_string(),
        "stdout is the path alone"
    );
    assert!(
        stderr.contains("peak bin"),
        "the report goes to stderr:\n{stderr}"
    );
    assert!(stderr.contains("rl_i16"));

    // --quiet says nothing at all but still writes the file.
    let quiet = run(&[
        input.to_str().unwrap(),
        "-f",
        "256",
        "-o",
        output.to_str().unwrap(),
        "-q",
    ]);
    assert!(quiet.status.success());
    assert!(quiet.stdout.is_empty() && quiet.stderr.is_empty());
    assert_png(&output);
}

#[test]
fn the_output_path_defaults_to_the_input_name() {
    let dir = TempDir::new("e2e-default-out");
    let sample_type = "rl_i16".parse().unwrap();
    let values = real_tone(SAMPLES, RATE as f64, TONE_HZ, 0.8);
    let input = write_wav(&dir.join("capture.iqw"), sample_type, RATE, &values, 1.0);

    let out = run(&[input.to_str().unwrap(), "-f", "256", "-q"]);
    assert!(out.status.success());
    assert_png(&dir.join("capture.iqw.png"));
}

#[test]
fn failures_are_reported_and_exit_non_zero() {
    let dir = TempDir::new("e2e-errors");

    let missing = run(&[dir.join("nope.wav").to_str().unwrap()]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("error:"));

    let mystery = dir.join("mystery.bin");
    std::fs::write(&mystery, [7u8; 4096]).unwrap();
    let unknown = run(&[mystery.to_str().unwrap()]);
    assert!(!unknown.status.success());
    let msg = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        msg.contains("--raw"),
        "the error should point at the fix:\n{msg}"
    );

    // A transform larger than the signal cannot produce a frame.
    let sample_type = "rl_i16".parse().unwrap();
    let short = write_wav(&dir.join("short.wav"), sample_type, RATE, &[0.1; 64], 1.0);
    let too_short = run(&[short.to_str().unwrap(), "-f", "2048"]);
    assert!(!too_short.status.success());
    assert!(String::from_utf8_lossy(&too_short.stderr).contains("2048"));
}

#[test]
fn real_captures_render_end_to_end() {
    // The repository's own fixture directory first, then an override, then
    // the sibling sgvr checkout the captures originally came from.
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut bases = vec![repo.join("tests/signals")];
    if let Ok(extra) = std::env::var("ARGAND_EXTRA_FIXTURES") {
        bases.push(PathBuf::from(extra));
    }
    bases.push(repo.join("../sgvr/cli/tests"));

    let cases = [
        ("iq_i16-hfdl.iqw", "iq_i16"),
        ("iq_f16x8-ntx.wav", "iq_f16x8"),
        ("rl_f32-hfdl.wav", "rl_f32"),
        ("iq_f32-ft8.flac", "iq_i16"),
        ("rl_f32-hfdl.flac", "rl_i16"),
    ];

    let dir = TempDir::new("e2e-real");
    let mut ran = 0;
    for (name, want_type) in cases {
        let Some(input) = bases.iter().map(|b| b.join(name)).find(|p| p.exists()) else {
            eprintln!("skipping: no fixture named {name}");
            continue;
        };
        let output = png(&dir, &format!("{name}.png"));
        let report = run_json(&[
            input.to_str().unwrap(),
            "--ref",
            "peak",
            "-d",
            "60",
            "-i",
            "640x360",
            "-o",
            output.to_str().unwrap(),
        ]);

        assert_eq!(report["sample_type"], want_type, "{name}");
        assert!(report["stft"]["frames"].as_u64().unwrap() > 0, "{name}");
        assert!(
            report["peak_bin"]["dbfs"].as_f64().unwrap().is_finite(),
            "{name}"
        );
        assert_png(&output);
        ran += 1;
    }
    eprintln!("rendered {ran} of {} real captures", cases.len());
}

#[test]
fn the_half_hour_capture_renders_end_to_end() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/signals/12.579000_25_08_26_06_09_10.iqw");
    if !input.exists() {
        eprintln!("skipping: no {}", input.display());
        return;
    }

    let out = TempDir::new("e2e-capture");
    let output = out.join("capture.png");
    let report = run_json(&[
        input.to_str().unwrap(),
        "--center",
        "12.579M",
        "--ref",
        "peak",
        "-d",
        "40",
        "-i",
        "800x300",
        "-o",
        output.to_str().unwrap(),
    ]);

    assert_eq!(report["sample_type"], "iq_i16");
    assert_eq!(report["samples"], 43_200_000u64);
    assert_eq!(report["duration_seconds"], 1800.0);
    assert!(report["stft"]["frames"].as_u64().unwrap() > 80_000);
    let freq = report["peak_bin"]["freq_hz"].as_f64().unwrap();
    assert!(
        (12_567_000.0..=12_591_000.0).contains(&freq),
        "peak at {freq} Hz is outside the captured band"
    );
    assert_png(&output);
}
