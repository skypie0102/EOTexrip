use anyhow::{Result, ensure};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

/// Raw scanlines preserve sample depth, palette and transparency when flipping
/// an edited legacy deployment. The editable master is not rewritten.
pub struct Image {
    info: png::Info<'static>,
    data: Vec<u8>,
    line_size: usize,
}
impl Image {
    pub fn read(path: &Path) -> Result<Self> {
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_limits(png::Limits {
            bytes: 512 * 1024 * 1024,
        });
        let mut reader = decoder.read_info()?;
        let info = reader.info().clone();
        ensure!(
            info.animation_control.is_none(),
            "animated PNGs are not texture masters"
        );
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| anyhow::anyhow!("PNG byte size overflow"))?;
        ensure!(
            size <= 512 * 1024 * 1024,
            "PNG exceeds orientation conversion budget"
        );
        let mut data = vec![0; size];
        let frame = reader.next_frame(&mut data)?;
        data.truncate(frame.buffer_size());
        Ok(Self {
            info,
            data,
            line_size: frame.line_size,
        })
    }
    pub fn rgba8_digest(&self, width: u32, height: u32) -> Option<String> {
        if self.info.width != width
            || self.info.height != height
            || self.info.bit_depth != png::BitDepth::Eight
            || self.info.color_type != png::ColorType::Rgba
        {
            return None;
        }
        let mut identity = Vec::with_capacity(self.data.len() + 8);
        identity.extend_from_slice(&width.to_le_bytes());
        identity.extend_from_slice(&height.to_le_bytes());
        identity.extend_from_slice(&self.data);
        Some(crate::catalog::digest(&identity))
    }
    pub fn flip_vertical(&mut self) {
        let height = self.info.height as usize;
        for y in 0..height / 2 {
            let (top, bottom) = self.data.split_at_mut((height - 1 - y) * self.line_size);
            top[y * self.line_size..(y + 1) * self.line_size]
                .swap_with_slice(&mut bottom[..self.line_size]);
        }
    }
    pub fn write(&self, path: &Path) -> Result<()> {
        let mut info = self.info.clone();
        // Decoder output is deinterlaced scanlines, including for Adam7 input.
        info.interlaced = false;
        let encoder = png::Encoder::with_info(BufWriter::new(File::create(path)?), info)?;
        encoder.write_header()?.write_image_data(&self.data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_preserves_packed_palette_samples_and_transparency() {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("indexed.png");
        let target = t.path().join("flipped.png");
        let mut encoder = png::Encoder::new(File::create(&source).unwrap(), 3, 3);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Two);
        encoder.set_palette(vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
        encoder.set_trns(vec![0, 64, 128, 255]);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[0x18, 0x6c, 0xe4])
            .unwrap();
        let before = std::fs::read(&source).unwrap();
        let mut image = Image::read(&source).unwrap();
        image.flip_vertical();
        image.write(&target).unwrap();
        let flipped = Image::read(&target).unwrap();
        assert_eq!(flipped.data, [0xe4, 0x6c, 0x18]);
        assert_eq!(flipped.info.bit_depth, png::BitDepth::Two);
        assert_eq!(flipped.info.color_type, png::ColorType::Indexed);
        assert_eq!(flipped.info.palette, image.info.palette);
        assert_eq!(flipped.info.trns, image.info.trns);
        assert_eq!(std::fs::read(source).unwrap(), before);
    }
}
