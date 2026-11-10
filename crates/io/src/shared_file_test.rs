use super::*;

#[test]
fn cursors_over_one_handle_keep_their_own_positions() {
    let dir = crate::testutil::TempDir::new("shared");
    let path = dir.join("bytes");
    std::fs::write(&path, b"0123456789").unwrap();
    let file = Arc::new(File::open(&path).unwrap());
    let mut a = SharedFile::new(file.clone()).unwrap();
    let mut b = SharedFile::new(file).unwrap();
    let mut buf = [0u8; 3];
    a.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"012");
    b.seek(SeekFrom::End(-2)).unwrap();
    b.read_exact(&mut buf[..2]).unwrap();
    assert_eq!(&buf[..2], b"89");
    a.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"345");
    assert!(a.seek(SeekFrom::Current(-10)).is_err());
    assert_eq!(a.byte_len(), Some(10));
}
