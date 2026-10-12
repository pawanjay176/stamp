//! Hardware-independent display contracts.

/// Physical pixel dimensions of a display surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayGeometry {
    pub width: u16,
    pub height: u16,
}

impl DisplayGeometry {
    /// Bytes needed for a row-major, one-bit-per-pixel framebuffer.
    pub const fn monochrome_buffer_len(self) -> usize {
        (self.width as usize).div_ceil(8) * self.height as usize
    }
}

/// The only refresh policy exposed before panel-specific fast modes are
/// understood and measured.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefreshMode {
    Full,
}

/// Minimal surface required by the reader core.
///
/// The X4 implementation will be written in the display phase. Keeping the
/// associated error type here avoids coupling the core to a particular HAL.
pub trait MonochromeDisplay {
    type Error;

    fn geometry(&self) -> DisplayGeometry;
    fn framebuffer(&mut self) -> &mut [u8];
    fn refresh(&mut self, mode: RefreshMode) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::DisplayGeometry;

    #[test]
    fn x4_framebuffer_is_48_000_bytes() {
        let geometry = DisplayGeometry {
            width: 800,
            height: 480,
        };

        assert_eq!(geometry.monochrome_buffer_len(), 48_000);
    }

    #[test]
    fn partial_bytes_are_rounded_up_per_row() {
        let geometry = DisplayGeometry {
            width: 9,
            height: 2,
        };

        assert_eq!(geometry.monochrome_buffer_len(), 4);
    }
}
