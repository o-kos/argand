//! One opened file read by several parsers, each at a position of its own.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use symphonia::core::io::MediaSource;

/// A cursor over a shared handle that reads at a position of its own, never the handle's.
pub(crate) struct SharedFile {
    file: Arc<File>,
    pos: u64,
    len: u64,
}

impl SharedFile {
    pub(crate) fn new(file: Arc<File>) -> std::io::Result<Self> {
        let len = file.metadata()?.len();
        Ok(Self { file, pos: 0, len })
    }
}

impl Read for SharedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = read_at(&self.file, buf, self.pos)?;
        self.pos += read as u64;
        Ok(read)
    }
}

impl Seek for SharedFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(delta) => self.len.checked_add_signed(delta),
            SeekFrom::Current(delta) => self.pos.checked_add_signed(delta),
        };
        let target = target.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before the start of the file",
            )
        })?;
        self.pos = target;
        Ok(target)
    }
}

impl MediaSource for SharedFile {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

/// `seek_read` moves the handle's own position, which no reader here relies on.
#[cfg(windows)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}

#[cfg(test)]
mod tests {
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
}
