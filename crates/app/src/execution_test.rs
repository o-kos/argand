use super::*;

#[test]
fn budgets_respect_process_limits_and_selected_cpus() {
    assert_eq!(worker_count(0, 12, None), 8);
    assert_eq!(worker_count(4, 12, None), 4);
    assert_eq!(worker_count(32, 12, None), 12);
    assert_eq!(worker_count(0, 1, None), 1);
    assert_eq!(worker_count(8, 12, Some(&[7, 19, 31])), 3);
    assert_eq!(worker_count(8, 2, Some(&[7, 19, 31])), 2);
}

#[test]
fn invalid_fields_are_repaired_independently() {
    let mut settings = Settings {
        workers: 999,
        batch_frames: 0,
        affinity: Affinity::Efficiency,
    };
    settings.repair();
    assert_eq!(settings.workers, 0);
    assert_eq!(settings.batch_frames, Settings::default().batch_frames);
    assert_eq!(settings.affinity, Affinity::Efficiency);
}
