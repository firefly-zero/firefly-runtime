use crate::frame_buffer::{HEIGHT, WIDTH};
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};

pub(crate) fn save_png<W, E>(mut w: W, palette: &[Rgb888; 16], frame: &[u8]) -> Result<(), E>
where
    W: embedded_io::Write<Error = E>,
{
    w.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;
    // png::Encoder::write_header(self)

    // Write IHDR chunk.
    let mut data: [u8; 13] = [0; 13];
    data[..4].copy_from_slice(&(WIDTH as u32).to_be_bytes());
    data[4..8].copy_from_slice(&(HEIGHT as u32).to_be_bytes());
    data[8] = 4; // bit depth: 4 BPP
    data[9] = 3; // color type: indexed (uses palette)
    data[12] = 0; // interlancing: no
    write_chunk(&mut w, b"IHDR", &data)?;
    write_chunk(&mut w, b"PLTE", &encode_palette(palette))?;
    write_chunk(&mut w, b"IDAT", frame)?;

    Ok(())
}

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
