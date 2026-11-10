use super::*;
use argand_io::testutil::TempDir;

#[test]
fn a_reopen_error_names_its_file() {
    let path = PathBuf::from("/captures/a.wav");
    let error = anyhow::Error::from(IoError::Source {
        path: path.clone(),
        source: SourceError::Changed,
    })
    .context("cannot draw it");
    assert_eq!(changed_file(&error), Some(Some(path)));
}

#[test]
fn a_reader_error_names_no_file() {
    let error = anyhow::Error::from(SourceError::Changed).context("transform failed");
    assert_eq!(changed_file(&error), Some(None));
}

#[test]
fn an_unnamed_change_takes_the_file_it_was_read_from() {
    let path = PathBuf::from("/captures/b.flac");
    let error = anyhow::Error::from(SourceError::Changed).context("minimap");
    assert_eq!(
        changed_file(&naming(error, &path)),
        Some(Some(path.clone()))
    );
    let other = anyhow::Error::from(IoError::Source {
        path: PathBuf::from("/captures/a.wav"),
        source: SourceError::Changed,
    });
    assert_eq!(
        changed_file(&naming(other, &path)),
        Some(Some(PathBuf::from("/captures/a.wav")))
    );
}

#[test]
fn other_errors_are_not_changes() {
    let error = anyhow::Error::from(SourceError::Decode("bad frame".into()));
    assert_eq!(changed_file(&error), None);
}

#[test]
fn a_rewritten_file_is_found() {
    let dir = TempDir::new("watched");
    let a = dir.join("a.raw");
    let b = dir.join("b.raw");
    std::fs::write(&a, [0u8; 8]).unwrap();
    std::fs::write(&b, [0u8; 8]).unwrap();
    let watched = Watched(vec![
        (a.clone(), SourceStamp::of(&a).unwrap()),
        (b.clone(), SourceStamp::of(&b).unwrap()),
    ]);
    assert_eq!(watched.first_changed(), None);
    let file = std::fs::OpenOptions::new().write(true).open(&b).unwrap();
    file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    drop(file);
    assert_eq!(watched.first_changed(), Some(b.clone()));
    std::fs::remove_file(&b).unwrap();
    assert_eq!(watched.first_changed(), Some(b));
}
