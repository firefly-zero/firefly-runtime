use crate::frame_buffer::{HEIGHT, WIDTH};
use core2::io::Write;
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};

/// Write the frame buffer as a PNG file.
pub(crate) fn save_png<W, E>(mut w: W, palette: &[Rgb888; 16], frame: &[u8]) -> Result<(), E>
where
    W: embedded_io::Write<Error = E>,
{
    w.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;
    let mut ihdr: [u8; 13] = [0; 13];
    ihdr[..4].copy_from_slice(&(WIDTH as u32).to_be_bytes());
    ihdr[4..8].copy_from_slice(&(HEIGHT as u32).to_be_bytes());
    ihdr[8] = 4; // bit depth: 4 BPP
    ihdr[9] = 3; // color type: indexed (uses palette)
    write_chunk(&mut w, b"IHDR", &ihdr)?;
    write_chunk(&mut w, b"PLTE", &encode_palette(palette))?;
    write_frame(&mut w, frame)?;
    write_chunk(&mut w, b"IEND", &[])?;
    Ok(())
}

/// Write the compressed PNG image data.
fn write_frame<W, E>(mut w: W, data: &[u8]) -> Result<(), E>
where
    W: embedded_io::Write<Error = E>,
{
    let inner = Buffer::new();
    let mut compressor = libflate::zlib::Encoder::new(inner).unwrap();
    for line in data.chunks(WIDTH / 2) {
        compressor.write_all(&[0]).unwrap(); // filter type: no filter
        compressor.write_all(&swap_pairs(line)).unwrap();
    }
    let compressed = compressor.finish().into_result().unwrap();
    write_chunk(&mut w, b"IDAT", &compressed.buf)?;
    Ok(())
}

/// Each byte in the frame buffer contains 2 pixels. Swap these 2 pixels.
fn swap_pairs(frame: &[u8]) -> alloc::vec::Vec<u8> {
    frame.iter().map(|byte| byte.rotate_left(4)).collect()
}

/// Serialize the palette as continius RGB bytes.
fn encode_palette(palette: &[Rgb888; 16]) -> [u8; 16 * 3] {
    let mut encoded: [u8; 16 * 3] = [0; 16 * 3];
    for (i, color) in palette.iter().enumerate() {
        let i = i * 3;
        encoded[i + 0] = color.r();
        encoded[i + 1] = color.g();
        encoded[i + 2] = color.b();
    }
    encoded
}

/// Write a PNG chunk.
fn write_chunk<W, E>(mut w: W, name: &[u8; 4], data: &[u8]) -> Result<(), E>
where
    W: embedded_io::Write<Error = E>,
{
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(name)?;
    w.write_all(data)?;
    let mut crc = crc32fast::Hasher::new();
    crc.update(name);
    crc.update(data);
    w.write_all(&crc.finalize().to_be_bytes())?;
    Ok(())
}

/// Adapter implementing core2 (used by libflate) writer for vector.
struct Buffer {
    buf: alloc::vec::Vec<u8>,
}

impl Buffer {
    fn new() -> Self {
        Self {
            buf: alloc::vec::Vec::new(),
        }
    }
}

impl core2::io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> core2::io::Result<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> core2::io::Result<()> {
        Ok(())
    }
}
