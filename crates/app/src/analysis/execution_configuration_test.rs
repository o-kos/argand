use super::*;

#[test]
fn partial_analysis_configuration_keeps_independent_defaults() {
    let config: crate::config::Config =
        toml::from_str("[analysis]\nworkers = 4\naffinity = 'efficiency'").unwrap();
    assert_eq!(config.analysis.workers, 4);
    assert_eq!(
        config.analysis.batch_frames,
        Settings::default().batch_frames
    );
    assert_eq!(config.analysis.affinity, Affinity::Efficiency);
    assert!(toml::from_str::<crate::config::Config>("[analysis]\naffinity = 'cpu4'").is_err());
}
