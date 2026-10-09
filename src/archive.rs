use crate::binary::*;
use anyhow::{Result, ensure};

pub const MAX_RESOURCE: usize = 256 * 1024 * 1024;
#[derive(Debug)]
pub struct Member {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub expanded_size: usize,
}

pub fn hpi(data: &[u8], hpb_size: u64) -> Result<Vec<Member>> {
    ensure!(bytes(data, 0, 4)? == b"HPIH", "not an HPI index");
    let buckets = u16le(data, 18)? as usize;
    let count = u16le(data, 20)? as usize;
    let table = 24 + buckets * 4;
    let names = table + count * 16;
    bytes(data, table, count * 16)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let at = table + i * 16;
        let name = logical_path(&cstring(data, names + u32le(data, at)? as usize, 4096)?)?;
        let offset = u32le(data, at + 4)? as usize;
        let size = u32le(data, at + 8)? as usize;
        let expanded_size = u32le(data, at + 12)? as usize;
        ensure!(
            (offset as u64)
                .checked_add(size as u64)
                .is_some_and(|n| n <= hpb_size),
            "HPI member {name} outside HPB"
        );
        ensure!(
            size <= MAX_RESOURCE && expanded_size <= MAX_RESOURCE,
            "HPI member {name} exceeds resource limit"
        );
        out.push(Member {
            name,
            offset,
            size,
            expanded_size,
        });
    }
    Ok(out)
}

pub fn acmp(data: &[u8]) -> Result<Vec<u8>> {
    ensure!(bytes(data, 0, 4)? == b"ACMP", "not ACMP");
    let compressed = u32le(data, 4)? as usize;
    let header = u32le(data, 8)? as usize;
    let size = u32le(data, 16)? as usize;
    ensure!(
        header >= 32 && size <= MAX_RESOURCE && compressed >= 8,
        "invalid ACMP dimensions"
    );
    let input = bytes(data, header, compressed)?;
    let tail = u32le(input, compressed - 8)?;
    let packed = (tail & 0xffffff) as usize;
    let trailer = (tail >> 24) as usize;
    let increase = u32le(input, compressed - 4)? as usize;
    ensure!(
        trailer >= 8 && trailer <= compressed && packed <= compressed && packed >= trailer,
        "invalid ACMP trailer"
    );
    let target = packed
        .checked_add(increase)
        .ok_or_else(|| anyhow::anyhow!("ACMP expansion overflow"))?;
    ensure!(target <= size, "ACMP trailer exceeds declared expansion");
    let mut output = vec![0; size];
    let mut read = compressed - trailer;
    let mut write = size;
    let stop = size - target;
    while write > stop {
        ensure!(read > 0, "truncated ACMP flags");
        read -= 1;
        let flags = input[read];
        for bit in (0..8).rev() {
            if write <= stop {
                break;
            }
            if flags & (1 << bit) == 0 {
                ensure!(read > 0 && write > 0, "truncated ACMP literal");
                read -= 1;
                write -= 1;
                output[write] = input[read];
            } else {
                ensure!(read >= 2, "truncated ACMP back-reference");
                read -= 1;
                let high = input[read];
                read -= 1;
                let low = input[read];
                let len = (high >> 4) as usize + 3;
                let distance = (((high & 15) as usize) << 8 | low as usize) + 3;
                ensure!(len <= write - stop, "ACMP run exceeds output");
                for _ in 0..len {
                    write -= 1;
                    ensure!(write + distance < size, "invalid ACMP backward reference");
                    output[write] = output[write + distance];
                }
            }
        }
    }
    ensure!(read == write, "ACMP literal prefix size mismatch");
    output[..write].copy_from_slice(&input[..read]);
    Ok(output)
}

pub fn nintendo_lz(data: &[u8]) -> Result<Vec<u8>> {
    ensure!(matches!(data.first(), Some(0x10 | 0x11)), "not Nintendo LZ");
    let mode = data[0];
    let mut size = (u32le(data, 0)? >> 8) as usize;
    let mut at = 4;
    if size == 0 && mode == 0x11 {
        size = u32le(data, 4)? as usize;
        at = 8;
    }
    ensure!(
        size > 0 && size <= MAX_RESOURCE,
        "Nintendo LZ expansion limit"
    );
    let mut output = Vec::with_capacity(size);
    while output.len() < size {
        let flags = *bytes(data, at, 1)?.first().unwrap();
        at += 1;
        for bit in (0..8).rev() {
            if output.len() == size {
                break;
            }
            if flags & (1 << bit) == 0 {
                output.push(*bytes(data, at, 1)?.first().unwrap());
                at += 1;
                continue;
            }
            let a = *bytes(data, at, 1)?.first().unwrap() as usize;
            let b = *bytes(data, at + 1, 1)?.first().unwrap() as usize;
            let (len, distance) = if mode == 0x10 {
                at += 2;
                ((a >> 4) + 3, ((a & 15) << 8 | b) + 1)
            } else {
                match a >> 4 {
                    0 => {
                        let c = bytes(data, at + 2, 1)?[0] as usize;
                        at += 3;
                        (((a & 15) << 4 | (b >> 4)) + 0x11, ((b & 15) << 8 | c) + 1)
                    }
                    1 => {
                        let c = bytes(data, at + 2, 1)?[0] as usize;
                        let d = bytes(data, at + 3, 1)?[0] as usize;
                        at += 4;
                        (
                            ((a & 15) << 12 | (b << 4) | (c >> 4)) + 0x111,
                            ((c & 15) << 8 | d) + 1,
                        )
                    }
                    n => {
                        at += 2;
                        (n + 1, ((a & 15) << 8 | b) + 1)
                    }
                }
            };
            ensure!(
                distance <= output.len() && output.len() + len <= size,
                "invalid Nintendo LZ back-reference"
            );
            for _ in 0..len {
                output.push(output[output.len() - distance]);
            }
        }
    }
    Ok(output)
}

pub fn farc(data: &[u8]) -> Result<Vec<Member>> {
    ensure!(bytes(data, 0, 4)? == b"FARC", "not FARC");
    let mut outer = vec![];
    let mut at = 36;
    let mut first = data.len();
    while at + 8 <= first {
        let offset = u32le(data, at)? as usize;
        let size = u32le(data, at + 4)? as usize;
        if offset == 0 && size == 0 {
            break;
        }
        ensure!(
            offset >= at + 8 && size <= MAX_RESOURCE,
            "invalid FARC table entry"
        );
        bytes(data, offset, size)?;
        first = first.min(offset);
        outer.push(Member {
            name: format!("member_{:05}", outer.len()),
            offset,
            size,
            expanded_size: 0,
        });
        at += 8;
        ensure!(outer.len() <= 65536, "FARC member limit exceeded");
    }
    ensure!(!outer.is_empty(), "empty or unsupported FARC table");
    if outer.len() == 2 {
        let fat = bytes(data, outer[0].offset, outer[0].size)?;
        if fat.starts_with(b"SIR0") {
            let sub = u32le(fat, 4)? as usize;
            let table = u32le(fat, sub)? as usize;
            let count = u32le(fat, sub + 4)? as usize;
            ensure!(count <= 65536, "SIR0 FAT member limit exceeded");
            bytes(fat, table, count * 12)?;
            let base = outer[1].offset;
            let length = outer[1].size;
            let mut out = vec![];
            for i in 0..count {
                let record = table + i * 12;
                let identity = u32le(fat, record)?;
                let offset = u32le(fat, record + 4)? as usize;
                let size = u32le(fat, record + 8)? as usize;
                ensure!(
                    offset.checked_add(size).is_some_and(|n| n <= length) && size <= MAX_RESOURCE,
                    "SIR0 FAT member out of bounds"
                );
                // Some tables store name pointers, others hashes. Only a real
                // terminated printable string is used as a source filename.
                let name = cstring(fat, identity as usize, 256)
                    .ok()
                    .filter(|s| s.contains('.') && s.chars().all(|c| !c.is_control()))
                    .and_then(|s| logical_path(&s).ok())
                    .unwrap_or_else(|| format!("id_{identity:08X}_{i:05}"));
                out.push(Member {
                    name,
                    offset: base + offset,
                    size,
                    expanded_size: 0,
                });
            }
            return Ok(out);
        }
    }
    Ok(outer)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acmp_backward_runs_preserve_literal_prefix_and_reject_invalid_history() {
        // A raw prefix followed by three literals and an overlapping 18-byte
        // backward reference. The compressed footer includes its eight bytes.
        let mut packed = b"PREFIX".to_vec();
        packed.extend_from_slice(&[0, 0xf0, b'a', b'b', b'c', 0x10]);
        packed.extend_from_slice(&(8u32 << 24 | 14).to_le_bytes());
        packed.extend_from_slice(&7u32.to_le_bytes());
        let mut data = vec![0; 32];
        data[..4].copy_from_slice(b"ACMP");
        data[4..8].copy_from_slice(&(packed.len() as u32).to_le_bytes());
        data[8..12].copy_from_slice(&32u32.to_le_bytes());
        data[16..20].copy_from_slice(&27u32.to_le_bytes());
        data.extend_from_slice(&packed);
        assert_eq!(acmp(&data).unwrap(), b"PREFIXabcabcabcabcabcabcabc");
        // With no preceding literals, a reference cannot read initialized
        // history. Failure must precede publication of an extracted resource.
        data[43] = 0x80;
        assert!(acmp(&data).is_err());
        data[43] = 0x10;
        data[48..52].copy_from_slice(&100u32.to_le_bytes());
        assert!(acmp(&data).is_err());
    }
    #[test]
    fn lz_literals_and_bounds() {
        assert_eq!(
            nintendo_lz(&[0x10, 3, 0, 0, 0, b'a', b'b', b'c']).unwrap(),
            b"abc"
        );
        assert!(nintendo_lz(&[0x10, 8, 0, 0, 0x80, 0, 0]).is_err());
        assert!(logical_path("../../escape").is_err());
        assert!(logical_path("C:\\escape").is_err());
    }
    #[test]
    fn farc_uses_structural_tables() {
        let mut data = vec![0; 132];
        data[..4].copy_from_slice(b"FARC");
        data[36..40].copy_from_slice(&128u32.to_le_bytes());
        data[40..44].copy_from_slice(&4u32.to_le_bytes());
        data[128..].copy_from_slice(b"TEST");
        let out = farc(&data).unwrap();
        assert_eq!(out[0].offset, 128);
        assert_eq!(out[0].size, 4);
        data[40..44].copy_from_slice(&5u32.to_le_bytes());
        assert!(farc(&data).is_err());
    }
}
