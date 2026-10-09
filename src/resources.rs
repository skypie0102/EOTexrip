use crate::{binary::*, pica};
use anyhow::{Result, bail, ensure};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Texture {
    pub name: String,
    pub parser: &'static str,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub payload: Vec<u8>,
    pub offset: usize,
}
#[allow(clippy::too_many_arguments)] // Named binary descriptor fields remain explicit.
fn texture(
    data: &[u8],
    name: String,
    parser: &'static str,
    width: u32,
    height: u32,
    format: u32,
    offset: usize,
    declared: usize,
) -> Result<Texture> {
    let size = pica::base_size(width, height, format)?;
    ensure!(
        declared >= size,
        "texture storage is shorter than its padded base mip"
    );
    bytes(data, offset, declared)?;
    Ok(Texture {
        name,
        parser,
        width,
        height,
        format,
        payload: bytes(data, offset, size)?.to_vec(),
        offset,
    })
}
pub fn stex(data: &[u8]) -> Result<Texture> {
    ensure!(bytes(data, 0, 4)? == b"STEX", "not STEX");
    let width = u32le(data, 12)?;
    let height = u32le(data, 16)?;
    let typ = u32le(data, 20)?;
    let code = u32le(data, 24)?;
    let format = match (code, typ) {
        (0x6752, 0x1401) => 0,
        (0x6754, 0x1401) => 1,
        (0x6752, 0x8034) => 2,
        (0x6754, 0x8363) => 3,
        (0x6752, 0x8033) => 4,
        (0x6758, 0x1401) => 5,
        (0x6756, 0x1401) => 8,
        (0x6757, 0x1401) => 7,
        (0x6758, 0x6760) => 9,
        (0x6757, 0x6761) => 10,
        (0x6756, 0x6761) => 11,
        (0x675a, _) => 12,
        (0x675b, _) => 13,
        (0..=13, _) => code,
        _ => bail!("unsupported STEX type/format {typ:#x}/{code:#x}"),
    };
    let size = u32le(data, 28)? as usize;
    let offset = u32le(data, 32)? as usize;
    ensure!(offset >= 36, "STEX payload overlaps its header");
    let name = if offset > 40 {
        cstring(data, 40, offset - 40).unwrap_or_default()
    } else {
        String::new()
    };
    texture(data, name, "stex", width, height, format, offset, size)
}
pub fn cgfx(data: &[u8]) -> Result<(Vec<Texture>, Vec<String>)> {
    ensure!(bytes(data, 0, 4)? == b"CGFX", "not CGFX");
    ensure!(u16le(data, 4)? == 0xfeff, "unsupported CGFX endianness");
    let declared = u32le(data, 12)? as usize;
    ensure!(
        declared >= 20 && declared <= data.len(),
        "invalid CGFX declared length"
    );
    let data = &data[..declared];
    let mut out = vec![];
    let mut issues = vec![];
    for (at, magic) in data.windows(4).enumerate().filter(|(_, m)| *m == b"TXOB") {
        let _ = magic;
        if at < 4 || u32le(data, at - 4)? != 0x20000011 {
            continue;
        } // Image TXOB only.
        let result = (|| -> Result<Texture> {
            let name = cstring(data, relative(data, at + 8)?, 512)?;
            let height = u32le(data, at + 0x14)?;
            let width = u32le(data, at + 0x18)?;
            let format = u32le(data, at + 0x30)?;
            let image = relative(data, at + 0x34)?;
            ensure!(
                u32le(data, image)? == height && u32le(data, image + 4)? == width,
                "CGFX image descriptor dimensions disagree"
            );
            let size = u32le(data, image + 8)? as usize;
            let offset = relative(data, image + 12)?;
            texture(data, name, "cgfx", width, height, format, offset, size)
        })();
        match result {
            Ok(t) => out.push(t),
            Err(e) => issues.push(format!("CGFX image at {at:#x}: {e}")),
        };
    }
    Ok((out, issues))
}
fn commands(data: &[u8]) -> Result<BTreeMap<u32, u32>> {
    let mut at = 0;
    let mut out = BTreeMap::new();
    while at + 8 <= data.len() {
        let value = u32le(data, at)?;
        let header = u32le(data, at + 4)?;
        let reg = header & 0xffff;
        let mask = (header >> 16) & 15;
        let extra = ((header >> 20) & 0x7ff) as usize;
        let consecutive = header >> 31 != 0;
        let count = extra + 1;
        bytes(data, at + 8, extra * 4)?;
        for i in 0..count {
            let target = reg + if consecutive { i as u32 } else { 0 };
            let param = if i == 0 {
                value
            } else {
                u32le(data, at + 8 + (i - 1) * 4)?
            };
            let bitmask = (0..4).fold(0u32, |m, b| {
                m | if mask & (1 << b) != 0 {
                    255 << (b * 8)
                } else {
                    0
                }
            });
            let prior = out.get(&target).copied().unwrap_or(0);
            out.insert(target, (prior & !bitmask) | (param & bitmask));
        }
        at = (at + 8 + extra * 4).div_ceil(8) * 8;
    }
    ensure!(at <= data.len(), "truncated GPU command padding");
    Ok(out)
}
pub fn bch(data: &[u8]) -> Result<(Vec<Texture>, Vec<String>)> {
    ensure!(bytes(data, 0, 4)? == b"BCH\0", "not BCH");
    let ext = data[4] >= 0x21;
    let content = u32le(data, 8)? as usize;
    let strings = u32le(data, 12)? as usize;
    let cmd = u32le(data, 16)? as usize;
    let raw = u32le(data, 20)? as usize;
    let lengths = if ext { 0x20 } else { 0x1c };
    let content_len = u32le(data, lengths)? as usize;
    let string_len = u32le(data, lengths + 4)? as usize;
    let command_len = u32le(data, lengths + 8)? as usize;
    let raw_len = u32le(data, lengths + 12)? as usize;
    let contents = bytes(data, content, content_len)?;
    let names = bytes(data, strings, string_len)?;
    let gpu = bytes(data, cmd, command_len)?;
    let image = bytes(data, raw, raw_len)?;
    let ptr = u32le(contents, 36)? as usize;
    let count = u32le(contents, 40)? as usize;
    ensure!(count <= 16384, "BCH texture count limit exceeded");
    bytes(contents, ptr, count * 4)?;
    let mut out = vec![];
    let mut issues = vec![];
    for i in 0..count {
        let entry = (|| -> Result<Texture> {
            let desc = u32le(contents, ptr + i * 4)? as usize;
            let name = cstring(names, u32le(contents, desc + 28)? as usize, 512)?;
            let declared_format = *bytes(contents, desc + 24, 1)?.first().unwrap() as u32;
            let mut found = None;
            for (unit, (dim_reg, type_reg, addr_reg)) in
                [(0x82, 0x8e, 0x85), (0x92, 0x96, 0x95), (0x9a, 0x9e, 0x9d)]
                    .into_iter()
                    .enumerate()
            {
                let off = u32le(contents, desc + unit * 8)? as usize;
                let words = u32le(contents, desc + unit * 8 + 4)? as usize;
                if words == 0 {
                    continue;
                }
                ensure!(words <= 65536, "BCH command count limit exceeded");
                let regs = commands(bytes(gpu, off, words * 4)?)?;
                let (Some(&dim), Some(&address)) = (regs.get(&dim_reg), regs.get(&addr_reg)) else {
                    continue;
                };
                let width = (dim >> 16) & 0x7ff;
                let height = dim & 0x7ff;
                let format = regs
                    .get(&type_reg)
                    .map(|x| x & 15)
                    .unwrap_or(declared_format);
                let size = pica::base_size(width, height, format)?;
                let offset = address as usize;
                let payload = bytes(image, offset, size)?.to_vec();
                found = Some(Texture {
                    name: name.clone(),
                    parser: "bch",
                    width,
                    height,
                    format,
                    payload,
                    offset: raw + offset,
                });
                break;
            }
            found.ok_or_else(|| {
                anyhow::anyhow!("BCH texture {name} has no complete texture command binding")
            })
        })();
        match entry {
            Ok(t) => out.push(t),
            Err(e) => issues.push(format!("BCH texture entry {i}: {e}")),
        };
    }
    Ok((out, issues))
}

pub fn bcfnt(data: &[u8]) -> Result<Vec<Texture>> {
    ensure!(bytes(data, 0, 4)? == b"CFNT", "not BCFNT");
    ensure!(u16le(data, 4)? == 0xfeff, "unsupported BCFNT endianness");
    let declared = u32le(data, 12)? as usize;
    let mut at = u16le(data, 6)? as usize;
    ensure!(
        declared <= data.len() && declared >= at,
        "invalid BCFNT file length"
    );
    let mut out = vec![];
    while at + 8 <= declared {
        let size = u32le(data, at + 4)? as usize;
        ensure!(size >= 8 && at + size <= declared, "invalid BCFNT block");
        if bytes(data, at, 4)? == b"TGLP" {
            let sheet_size = u32le(data, at + 12)? as usize;
            // TGLP: cell parameters +8..11; sheet size +12; count +16;
            // format +18; dimensions +24/+26; absolute sheet pointer +28.
            let sheets = u16le(data, at + 16)? as usize;
            let format = u16le(data, at + 18)? as u32;
            let width = u16le(data, at + 24)? as u32;
            let height = u16le(data, at + 26)? as u32;
            let start = u32le(data, at + 28)? as usize;
            ensure!(sheets <= 4096, "font sheet count limit exceeded");
            for i in 0..sheets {
                out.push(texture(
                    data,
                    format!("font_sheet_{i:03}"),
                    "bcfnt",
                    width,
                    height,
                    format,
                    start + i * sheet_size,
                    sheet_size,
                )?);
            }
        }
        at += size;
    }
    Ok(out)
}

/// Standard CTPK packages only. Cube maps and disguised non-CTPK resources
/// remain explicit issues instead of being decoded using guessed offsets.
pub fn ctpk(data: &[u8]) -> Result<(Vec<Texture>, Vec<String>)> {
    ensure!(bytes(data, 0, 4)? == b"CTPK", "not CTPK");
    let count = u16le(data, 6)? as usize;
    ensure!(count > 0 && count <= 16384, "invalid CTPK texture count");
    let start = u32le(data, 8)? as usize;
    let section_size = u32le(data, 12)? as usize;
    bytes(data, 32, count * 32)?;
    bytes(data, start, section_size)?;
    ensure!(start >= 32 + count * 32, "CTPK pixels overlap entry table");
    let mut out = vec![];
    let mut issues = vec![];
    for i in 0..count {
        let parsed = (|| -> Result<Texture> {
            let at = 32 + i * 32;
            let name = cstring(data, u32le(data, at)? as usize, 512)?;
            let size = u32le(data, at + 4)? as usize;
            let offset = u32le(data, at + 8)? as usize;
            ensure!(
                offset.checked_add(size).is_some_and(|n| n <= section_size),
                "CTPK entry outside pixel section"
            );
            ensure!(
                bytes(data, at + 21, 1)?[0] == 2,
                "only 2D CTPK entries are supported"
            );
            texture(
                data,
                name,
                "ctpk",
                u16le(data, at + 16)? as u32,
                u16le(data, at + 18)? as u32,
                u32le(data, at + 12)?,
                start + offset,
                size,
            )
        })();
        match parsed {
            Ok(t) => out.push(t),
            Err(e) => issues.push(format!("CTPK entry {i}: {e}")),
        };
    }
    Ok((out, issues))
}

/// Only recognized, structurally validated formats are decoded. Scanning finds
/// embedded model/STEX resources without reinterpreting arbitrary pixel-like
/// bytes. The caller records every recognized malformed container.
pub fn scan(data: &[u8]) -> (Vec<Texture>, Vec<String>) {
    let mut out = vec![];
    let mut issues = vec![];
    let mut covered = 0;
    for at in 0..data.len().saturating_sub(3) {
        if at < covered {
            continue;
        }
        let magic = &data[at..at + 4];
        let parsed = match magic {
            b"STEX" => stex(&data[at..]).map(|x| (vec![x], vec![])),
            b"CGFX" => cgfx(&data[at..]),
            b"BCH\0" => bch(&data[at..]),
            b"CFNT" => bcfnt(&data[at..]).map(|x| (x, vec![])),
            b"CTPK" => ctpk(&data[at..]),
            _ => continue,
        };
        match parsed {
            Ok((textures, entry_issues)) => {
                issues.extend(
                    entry_issues
                        .into_iter()
                        .take(100)
                        .map(|e| format!("container at {at:#x}: {e}")),
                );
                if textures.is_empty() {
                    issues.push(format!(
                        "recognized {:?} at {at:#x} contained no supported image entries",
                        String::from_utf8_lossy(magic)
                    ));
                }
                let max = textures
                    .iter()
                    .map(|t| at + t.offset + t.payload.len())
                    .max()
                    .unwrap_or(at + 4);
                covered = max;
                for mut t in textures {
                    t.offset += at;
                    out.push(t);
                }
            }
            Err(e) => {
                if issues.len() < 100 {
                    issues.push(format!(
                        "{:?} at {at:#x}: {e}",
                        String::from_utf8_lossy(magic)
                    ));
                }
            }
        }
        if out.len() > 32768 {
            issues.push("resource texture-count limit exceeded".into());
            break;
        }
    }
    (out, issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(d: &mut [u8], off: usize, v: u32) {
        d[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[test]
    fn stex_combined_gl_type_not_format_guessing() {
        let mut d = vec![0; 128 + 128];
        d[..4].copy_from_slice(b"STEX");
        word(&mut d, 12, 8);
        word(&mut d, 16, 8);
        word(&mut d, 20, 0x8033);
        word(&mut d, 24, 0x6752);
        word(&mut d, 28, 128);
        word(&mut d, 32, 128);
        assert_eq!(stex(&d).unwrap().format, 4);
        word(&mut d, 28, 127);
        assert!(stex(&d).is_err());
    }
    #[test]
    fn no_false_texture_on_random_payload() {
        assert!(scan(&vec![127; 8192]).0.is_empty());
    }
    #[test]
    fn gpu_consecutive_and_partial_masks() {
        let mut d = vec![0; 16];
        word(&mut d, 0, 8);
        word(&mut d, 4, 0x801f0082);
        word(&mut d, 8, 16);
        let r = commands(&d).unwrap();
        assert_eq!(r.get(&0x82), Some(&8));
        assert_eq!(r.get(&0x83), Some(&16));
    }
}
