//! Small storage contracts designed around bounded caller-owned buffers.

/// Random-access reader used by the future ZIP layer.
///
/// Implementations return `Ok(0)` at end of file. Callers own the buffer, so a
/// book cannot choose how much RAM a read allocates.
pub trait ReadAt {
    type Error;

    fn len(&mut self) -> Result<u64, Self::Error>;
    fn read_at(&mut self, offset: u64, output: &mut [u8]) -> Result<usize, Self::Error>;
}
