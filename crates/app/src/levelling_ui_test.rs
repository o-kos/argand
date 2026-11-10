use super::super::replace_ui::tests::{open_window, wait_until};
use super::*;
use crate::editing::{Editing, Placement};
use argand_core::{Adjustment, SampleSpan};
use gpui_kit::TestAppContext;

/// A 16-bit I/Q capture of a 3 kHz tone at a quarter of full scale, opened from a fresh folder and described.
fn opened(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    label: &str,
) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("argand-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("capture.wav");
    let iq_i16: argand_core::SampleType = "iq_i16".parse().unwrap();
    let values = argand_io::testutil::iq_tone(48_000, 24_000.0, 3_000.0, 0.25);
    argand_io::testutil::write_wav(&path, iq_i16, 24_000, &values, 1.0);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx);
    });
    wait_until(cx, shell, "the file to describe itself", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described() && editing.full_scale().is_some())
    });
    path
}

fn read(path: &std::path::Path) -> Vec<f32> {
    let mut source = argand_io::open(path, &argand_io::OpenHints::default()).unwrap();
    let mut values = vec![0.0; source.meta().len_samples as usize * 2];
    let mut filled = 0;
    while filled < values.len() {
        filled += source.read(&mut values[filled..]).unwrap();
    }
    values
}

fn adjustment(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Adjustment {
    shell.read_with(cx, |shell, _| shell.editing().unwrap().adjustment())
}

#[gpui_kit::test]
fn inverting_is_refused_with_a_selection_and_saved_conjugated_without_one(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened(cx, &shell, "invert-ui");
    let selections = [
        (SampleSpan::between(100, 200), None),
        (None, argand_core::FrequencyBand::between(1_000.0, 2_000.0)),
        (
            SampleSpan::between(100, 200),
            argand_core::FrequencyBand::between(1_000.0, 2_000.0),
        ),
    ];
    for (time, band) in selections {
        shell.update(cx, |shell, _| {
            shell.selection = time;
            shell.band = band;
        });
        assert!(!shell.read_with(cx, |shell, _| shell.edit_commands_available().invert));
        shell.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(editing_ui::InvertSpectrum), cx);
        });
        cx.run_until_parked();
        assert!(adjustment(cx, &shell).is_identity(), "{time:?} {band:?}");
    }
    shell.update(cx, |shell, _| {
        shell.selection = None;
        shell.band = None;
    });
    let copied = shell.update(cx, |shell, _| {
        shell
            .editing()
            .unwrap()
            .copy(SampleSpan::between(0, 10).unwrap())
    });
    assert!(shell.read_with(cx, |shell, _| shell.edit_commands_available().invert));
    shell.update_in(cx, |_, window, cx| {
        window.dispatch_action(Box::new(editing_ui::InvertSpectrum), cx);
    });
    cx.run_until_parked();
    assert!(adjustment(cx, &shell).invert());
    shell.read_with(cx, |shell, _| {
        let editing = shell.editing().unwrap();
        assert!(editing.is_dirty() && !editing.is_untouched());
        assert_eq!(editing.capture().pieces().len(), 1);
        assert!(shell.edit_commands_available().inverted);
    });
    let refused = shell.update(cx, |shell, _| {
        let span = SampleSpan::between(0, 10).unwrap();
        shell
            .file
            .as_mut()
            .unwrap()
            .editing
            .as_mut()
            .unwrap()
            .paste(&copied, Placement::Replace(span))
    });
    assert_eq!(refused, Err(crate::editing::PasteError::Adjustment));
    shell.update_in(cx, |_, window, cx| {
        window.dispatch_action(Box::new(editing_ui::Undo), cx);
    });
    cx.run_until_parked();
    assert!(adjustment(cx, &shell).is_identity());
    shell.update_in(cx, |_, window, cx| {
        window.dispatch_action(Box::new(editing_ui::Redo), cx);
    });
    cx.run_until_parked();
    assert!(adjustment(cx, &shell).invert());

    let before = read(&path);
    shell.update_in(cx, |shell, window, cx| shell.save_over(window, cx));
    wait_until(cx, &shell, "the saved file to open again", |shell| {
        shell.saving.is_none()
            && shell
                .editing()
                .is_some_and(|editing| !editing.is_dirty() && editing.adjustment().is_identity())
    });
    let after = read(&path);
    for (n, (before, after)) in before
        .as_chunks::<2>()
        .0
        .iter()
        .zip(after.as_chunks::<2>().0.iter())
        .enumerate()
    {
        assert_eq!(after[0], before[0], "I of sample {n}");
        let expected = (-before[1]).min(32_767.0 / 32_768.0);
        assert_eq!(after[1], expected, "Q of sample {n}");
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[gpui_kit::test]
fn normalizing_measures_the_peak_and_saves_it_just_below_full_scale(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened(cx, &shell, "normalize-ui");
    shell.update_in(cx, |shell, window, cx| {
        shell.band = argand_core::FrequencyBand::between(1_000.0, 2_000.0);
        shell.normalize(window, cx);
    });
    assert!(shell.read_with(cx, |shell, _| {
        shell.levelling.is_none() && !shell.edit_commands_available().normalize
    }));
    shell.update_in(cx, |shell, window, cx| {
        shell.band = None;
        shell.normalize(window, cx);
    });
    assert!(shell.read_with(cx, |shell, _| {
        shell.levelling.is_some()
            && !shell.can_save(false)
            && shell.can_save_over().is_err()
            && shell.edit_commands_available() == editing_ui::EditCommands::default()
    }));
    wait_until(cx, &shell, "the level to be measured", |shell| {
        shell.levelling.is_none() && shell.editing().is_some_and(Editing::is_dirty)
    });
    let gain = adjustment(cx, &shell).gain();
    let expected = 4.0 * 10f64.powf(crate::levelling::TARGET / 20.0);
    assert!((gain - expected).abs() < 1e-3, "{gain}");
    assert_eq!(
        adjustment_text(adjustment(cx, &shell)).as_deref(),
        Some("+11.9 dB")
    );
    let versions = shell.read_with(cx, |shell, _| shell.editing().unwrap().version());
    shell.update_in(cx, |shell, window, cx| {
        shell.normalize(window, cx);
    });
    wait_until(cx, &shell, "the level to be measured again", |shell| {
        shell.levelling.is_none()
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().version()),
        versions,
        "the same level adds no version"
    );

    let target = path.with_file_name("saved.wav");
    shell.update_in(cx, |shell, window, cx| {
        let file = shell.file.as_ref().unwrap();
        let editing = file.editing.as_ref().unwrap();
        let request = editing.save_request(None, target.clone()).unwrap();
        let of = saving_ui::SaveOf {
            document: file.id,
            version: editing.version(),
            whole: false,
            rebind: false,
            then: None,
            over: false,
            _scratch: editing.scratches(),
        };
        shell.start_save(request, of, window, cx);
    });
    wait_until(cx, &shell, "the save to end", |shell| {
        shell.saving.is_none()
    });
    let peak = read(&target)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|sample| f64::from(sample[0]).hypot(f64::from(sample[1])))
        .fold(0.0, f64::max);
    assert!(
        (peak - 10f64.powf(crate::levelling::TARGET / 20.0)).abs() < 1e-3,
        "{peak}"
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[gpui_kit::test]
fn opening_another_file_drops_a_measurement_in_progress(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened(cx, &shell, "normalize-ui-open");
    shell.update_in(cx, |shell, window, cx| {
        shell.normalize(window, cx);
        shell.open(Origin::new(path.clone()), window, cx);
    });
    assert!(shell.read_with(cx, |shell, _| shell.levelling.is_none()));
    wait_until(cx, &shell, "the file to open again", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_untouched()));
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
