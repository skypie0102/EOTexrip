use anyhow::{Result, ensure};

pub const FORMAT_NAMES: [&str; 14] = [
    "RGBA8", "RGB8", "RGBA5551", "RGB565", "RGBA4", "LA8", "HILO8", "L8", "A8", "LA4", "L4", "A4",
    "ETC1", "ETC1A4",
];
const BITS: [usize; 14] = [32, 24, 16, 16, 16, 16, 16, 8, 8, 8, 4, 4, 4, 8];
pub fn base_size(width: u32, height: u32, format: u32) -> Result<usize> {
    ensure!(
        width > 0 && height > 0 && width <= 4096 && height <= 4096,
        "invalid PICA dimensions {width}x{height}"
    );
    let bits = *BITS
        .get(format as usize)
        .ok_or_else(|| anyhow::anyhow!("unsupported PICA format {format}"))?;
    Ok((width as usize).div_ceil(8) * 8 * (height as usize).div_ceil(8) * 8 * bits / 8)
}
fn morton(x: usize, y: usize) -> usize {
    (x & 1) | ((y & 1) << 1) | ((x & 2) << 1) | ((y & 2) << 2) | ((x & 4) << 2) | ((y & 4) << 3)
}
fn expand5(x: u16) -> u8 {
    ((x << 3) | (x >> 2)) as u8
}
fn pixel(data: &[u8], index: usize, format: u32) -> [u8; 4] {
    let at = index * BITS[format as usize] / 8;
    let v = || u16::from_le_bytes([data[at], data[at + 1]]);
    match format {
        0 => [data[at + 3], data[at + 2], data[at + 1], data[at]],
        1 => [data[at + 2], data[at + 1], data[at], 255],
        2 => {
            let v = v();
            [
                expand5(v >> 11),
                expand5((v >> 6) & 31),
                expand5((v >> 1) & 31),
                (v & 1) as u8 * 255,
            ]
        }
        3 => {
            let v = v();
            [
                expand5(v >> 11),
                ((((v >> 5) & 63) << 2) | (((v >> 5) & 63) >> 4)) as u8,
                expand5(v & 31),
                255,
            ]
        }
        4 => {
            let v = v();
            [
                ((v >> 12) & 15) as u8 * 17,
                ((v >> 8) & 15) as u8 * 17,
                ((v >> 4) & 15) as u8 * 17,
                (v & 15) as u8 * 17,
            ]
        }
        5 => [data[at + 1], data[at + 1], data[at + 1], data[at]],
        6 => [data[at + 1], data[at], 0, 255],
        7 => [data[at], data[at], data[at], 255],
        8 => [255, 255, 255, data[at]],
        9 => {
            let l = (data[at] >> 4) * 17;
            [l, l, l, (data[at] & 15) * 17]
        }
        10 => {
            let l = ((data[at] >> ((index & 1) * 4)) & 15) * 17;
            [l, l, l, 255]
        }
        11 => [255, 255, 255, ((data[at] >> ((index & 1) * 4)) & 15) * 17],
        _ => unreachable!(),
    }
}
fn etc_block(block: &[u8]) -> Result<[[u8; 3]; 16]> {
    let bits = u64::from_le_bytes(block.try_into()?);
    let hi = (bits >> 32) as u32;
    let lo = bits as u32;
    let mut colors = [[0i32; 3]; 2];
    for (channel, shift) in [28, 20, 12].into_iter().enumerate() {
        if hi & 2 == 0 {
            colors[0][channel] = ((hi >> shift) & 15) as i32 * 17;
            colors[1][channel] = ((hi >> (shift - 4)) & 15) as i32 * 17;
        } else {
            let base = ((hi >> (shift - 1)) & 31) as i32;
            let delta = ((hi >> (shift - 4)) & 7) as i32;
            let next = base + if delta & 4 != 0 { delta - 8 } else { delta };
            ensure!((0..=31).contains(&next), "invalid ETC1 differential color");
            colors[0][channel] = (base << 3) | (base >> 2);
            colors[1][channel] = (next << 3) | (next >> 2);
        }
    }
    let table = [
        (2, 8),
        (5, 17),
        (9, 29),
        (13, 42),
        (18, 60),
        (24, 80),
        (33, 106),
        (47, 183),
    ];
    let tables = [((hi >> 5) & 7) as usize, ((hi >> 2) & 7) as usize];
    let mut out = [[0; 3]; 16];
    for (j, p) in out.iter_mut().enumerate() {
        let x = j / 4;
        let y = j % 4;
        let sub = usize::from(if hi & 1 == 0 { x >= 2 } else { y >= 2 });
        let (small, large) = table[tables[sub]];
        let magnitude = if (lo >> j) & 1 == 0 { small } else { large };
        let adjustment = if (lo >> (j + 16)) & 1 == 0 {
            magnitude
        } else {
            -magnitude
        };
        for (c, value) in p.iter_mut().enumerate() {
            *value = (colors[sub][c] + adjustment).clamp(0, 255) as u8;
        }
    }
    Ok(out)
}

/// Untile to the source artwork's PNG row order. Azahar applies the GPU row
/// reversal on loading (`flip_png_files=true`); editable PNGs must not include
/// that reversal. Storage byte spans still include full padded 8x8 tiles.
pub fn decode(data: &[u8], width: u32, height: u32, format: u32) -> Result<Vec<u8>> {
    let required = base_size(width, height, format)?;
    ensure!(
        data.len() >= required,
        "short PICA payload: {} bytes, {required} required",
        data.len()
    );
    let w = width as usize;
    let h = height as usize;
    let tx = w.div_ceil(8);
    let mut out = vec![0; w * h * 4];
    if format < 12 {
        for y in 0..h {
            for x in 0..w {
                let index = ((y / 8) * tx + x / 8) * 64 + morton(x % 8, y % 8);
                let pixel = pixel(data, index, format);
                let at = (y * w + x) * 4;
                out[at..at + 4].copy_from_slice(&pixel);
            }
        }
    } else {
        let stride = if format == 13 { 16 } else { 8 };
        for my in 0..h.div_ceil(8) {
            for mx in 0..tx {
                for sub in 0..4 {
                    let at = ((my * tx + mx) * 4 + sub) * stride;
                    let alpha = if format == 13 {
                        u64::from_le_bytes(data[at..at + 8].try_into()?)
                    } else {
                        u64::MAX
                    };
                    let rgb_at = at + if format == 13 { 8 } else { 0 };
                    let rgb = etc_block(&data[rgb_at..rgb_at + 8])?;
                    for (j, pixel) in rgb.into_iter().enumerate() {
                        let x = mx * 8 + (sub % 2) * 4 + j / 4;
                        let y = my * 8 + (sub / 2) * 4 + j % 4;
                        if x < w && y < h {
                            let dest = (y * w + x) * 4;
                            out[dest..dest + 3].copy_from_slice(&pixel);
                            out[dest + 3] = ((alpha >> (j * 4)) & 15) as u8 * 17;
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

pub fn runtime_hash(payload: &[u8]) -> String {
    format!("{:016X}", cityhasher::hash::<u64>(payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cityhash_reference_vectors() {
        assert_eq!(runtime_hash(b""), "9AE16A3B2F90404F");
        assert_eq!(runtime_hash(b"hello"), "B48BE5A931380CE8");
    }
    #[test]
    fn channel_order_alpha_and_padding() {
        let mut data = vec![0u8; base_size(9, 5, 0).unwrap()];
        data[..4].copy_from_slice(&[80, 30, 20, 10]);
        let out = decode(&data, 9, 5, 0).unwrap();
        assert_eq!(&out[..4], &[10, 20, 30, 80]);
        assert_eq!(base_size(9, 5, 0).unwrap(), 512);
        assert!(decode(&data[..511], 9, 5, 0).is_err());
        let out = decode(&[1, 2].repeat(64), 8, 8, 6).unwrap();
        assert_eq!(&out[..4], &[2, 1, 0, 255]);
    }
    #[test]
    fn all_formats_require_complete_storage_and_decode() {
        for fmt in 0..14 {
            let bytes = vec![0; base_size(8, 8, fmt).unwrap()];
            let out = decode(&bytes, 8, 8, fmt).unwrap();
            assert_eq!(out.len(), 256);
            assert!(decode(&bytes[..bytes.len() - 1], 8, 8, fmt).is_err());
        }
        let mut bytes = vec![0; 64];
        bytes[..8].copy_from_slice(&0xFEDCBA9876543210u64.to_le_bytes());
        let out = decode(&bytes, 8, 8, 13).unwrap();
        assert_eq!(out[3], 0);
        assert_eq!(out[8 * 4 + 3], 17);
        assert_eq!(out[4 + 3], 68);
    }
    #[test]
    fn artwork_rows_stay_upright_in_every_format_and_across_tile_rows() {
        // Independent source-storage positions: (0,7)=42, (0,15)=106,
        // and (8,8)=192 in padded 8x8 Morton tiles.
        type RowCase = (&'static [u8], &'static [u8], [u8; 4], [u8; 4]);
        let raw_cases: &[RowCase] = &[
            (
                &[255, 3, 2, 1],
                &[255, 6, 5, 4],
                [1, 2, 3, 255],
                [4, 5, 6, 255],
            ),
            (&[3, 2, 1], &[6, 5, 4], [1, 2, 3, 255], [4, 5, 6, 255]),
            (&[1, 248], &[63, 0], [255, 0, 0, 255], [0, 0, 255, 255]),
            (&[0, 248], &[31, 0], [255, 0, 0, 255], [0, 0, 255, 255]),
            (&[15, 240], &[255, 0], [255, 0, 0, 255], [0, 0, 255, 255]),
            (
                &[255, 200],
                &[255, 20],
                [200, 200, 200, 255],
                [20, 20, 20, 255],
            ),
            (&[2, 1], &[5, 4], [1, 2, 0, 255], [4, 5, 0, 255]),
            (&[200], &[20], [200, 200, 200, 255], [20, 20, 20, 255]),
            (&[200], &[20], [255, 255, 255, 200], [255, 255, 255, 20]),
            (&[248], &[24], [255, 255, 255, 136], [17, 17, 17, 136]),
            (&[1], &[14], [17, 17, 17, 255], [238, 238, 238, 255]),
            (&[1], &[14], [255, 255, 255, 17], [255, 255, 255, 238]),
        ];
        for (width, height, bottom_index, bottom_x) in
            [(8, 8, 42, 0), (8, 16, 106, 0), (9, 9, 192, 8)]
        {
            for (format, &(top, bottom, top_rgba, bottom_rgba)) in raw_cases.iter().enumerate() {
                let mut bytes = vec![0; base_size(width, height, format as u32).unwrap()];
                bytes[..top.len()].copy_from_slice(top);
                let at = bottom_index * BITS[format] / 8;
                bytes[at..at + bottom.len()].copy_from_slice(bottom);
                let decoded = decode(&bytes, width, height, format as u32).unwrap();
                assert_eq!(
                    &decoded[..4],
                    &top_rgba,
                    "{} top {width}x{height}",
                    FORMAT_NAMES[format]
                );
                let at = (((height - 1) * width + bottom_x) * 4) as usize;
                assert_eq!(
                    &decoded[at..at + 4],
                    &bottom_rgba,
                    "{} bottom {width}x{height}",
                    FORMAT_NAMES[format]
                );
            }
            for format in [12, 13] {
                let stride = if format == 13 { 16 } else { 8 };
                let mut bytes = vec![0; base_size(width, height, format).unwrap()];
                for (block, data) in bytes.chunks_exact_mut(stride).enumerate() {
                    let bottom = match (width, height) {
                        (8, 8) => block >= 2,
                        (8, 16) => block >= 4,
                        (9, 9) => block >= 8,
                        _ => unreachable!(),
                    };
                    let color = if bottom { 0x0000f000u64 } else { 0xf0000000u64 } << 32;
                    let rgb_at = if format == 13 {
                        data[..8].fill(if bottom { 0x11 } else { 0xff });
                        8
                    } else {
                        0
                    };
                    data[rgb_at..rgb_at + 8].copy_from_slice(&color.to_le_bytes());
                }
                let decoded = decode(&bytes, width, height, format).unwrap();
                assert_eq!(&decoded[..4], &[255, 2, 2, 255]);
                let at = (((height - 1) * width + bottom_x) * 4) as usize;
                assert_eq!(
                    &decoded[at..at + 4],
                    &[2, 2, 255, if format == 13 { 17 } else { 255 }]
                );
            }
        }
    }
    #[test]
    fn tiles_do_not_shift_after_partial_dimensions() {
        let bytes = vec![0; base_size(9, 5, 12).unwrap()];
        assert_eq!(bytes.len(), 64);
        assert_eq!(decode(&bytes, 9, 5, 12).unwrap().len(), 9 * 5 * 4);
    }
}
