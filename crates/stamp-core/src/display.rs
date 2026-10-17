//! Hardware-independent display contracts.

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
/// An owned, allocation-free monochrome framebuffer backed by `[u8; N]`.
/// Set N to `ceil(WIDTH / 8) * HEIGHT`; the X4 uses 800, 480, and 48_000.
/// Pixel bits are packed most-significant first. For our panel, BinaryColor::On
/// (bit 1) is white and BinaryColor::Off (bit 0) is black.
///
/// This already implements embedded-graphics DrawTarget. Drawing changes RAM
/// only; the board driver must upload `data()` and request a refresh separately.
/// `new()` starts with zero bits (black). Clear to white before drawing black text.
pub type MonochromeFramebuffer<const WIDTH: usize, const HEIGHT: usize, const N: usize> =
    embedded_graphics::framebuffer::Framebuffer<
        embedded_graphics::pixelcolor::BinaryColor,
        embedded_graphics::pixelcolor::raw::RawU1,
        embedded_graphics::pixelcolor::raw::BigEndian,
        WIDTH,
        HEIGHT,
        N,
    >;

pub fn draw_message(
    framebuffer: &mut MonochromeFramebuffer<800, 480, 48000>,
) -> Result<(), core::convert::Infallible> {
    framebuffer.clear(BinaryColor::On)?;
    let text_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::Off);
    Text::new("Journey before Destination", Point::new(20, 40), text_style).draw(framebuffer)?;
    Ok(())
}

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
