#![allow(dead_code)]
pub fn word(d: &mut [u8], at: usize, v: u32) {
    d[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
pub fn short(d: &mut [u8], at: usize, v: u16) {
    d[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
pub fn long(d: &mut [u8], at: usize, v: u64) {
    d[at..at + 8].copy_from_slice(&v.to_le_bytes());
}
pub fn stex(name: &str, seed: u8, mips: usize) -> Vec<u8> {
    let mut d = vec![0; 128 + 256 + mips];
    d[..4].copy_from_slice(b"STEX");
    word(&mut d, 12, 8);
    word(&mut d, 16, 8);
    word(&mut d, 20, 0x1401);
    word(&mut d, 24, 0x6752);
    word(&mut d, 28, (256 + mips) as u32);
    word(&mut d, 32, 128);
    d[40..40 + name.len()].copy_from_slice(name.as_bytes());
    for i in 0..64 {
        d[128 + i * 4..132 + i * 4].copy_from_slice(&[255, seed, 20, i as u8]);
    }
    d
}
pub fn hpi(entries: &[(&str, Vec<u8>)]) -> (Vec<u8>, Vec<u8>) {
    let mut index = vec![0; 24 + entries.len() * 16];
    index[..4].copy_from_slice(b"HPIH");
    short(&mut index, 20, entries.len() as u16);
    let mut names = vec![];
    let mut payload = vec![];
    for (i, (name, data)) in entries.iter().enumerate() {
        let at = 24 + i * 16;
        word(&mut index, at, names.len() as u32);
        word(&mut index, at + 4, payload.len() as u32);
        word(&mut index, at + 8, data.len() as u32);
        names.extend_from_slice(name.as_bytes());
        names.push(0);
        payload.extend_from_slice(data);
    }
    index.extend(names);
    (index, payload)
}
pub fn cgfx(bad_second: bool) -> Vec<u8> {
    let n = if bad_second { 2 } else { 1 };
    let mut d = vec![0; 0x800 + n * 256];
    d[..4].copy_from_slice(b"CGFX");
    short(&mut d, 4, 0xfeff);
    short(&mut d, 6, 0x14);
    let len = d.len() as u32;
    word(&mut d, 12, len);
    for i in 0..n {
        let flag = 0x20 + i * 0x40;
        let at = flag + 4;
        word(&mut d, flag, 0x20000011);
        d[at..at + 4].copy_from_slice(b"TXOB");
        let name = 0x200 + i * 64;
        let image = 0x400 + i * 32;
        let raw = 0x800 + i * 256;
        word(&mut d, at + 8, (name - at - 8) as u32);
        let label = format!("en_goldhorn_t{i:02}");
        d[name..name + label.len()].copy_from_slice(label.as_bytes());
        word(&mut d, at + 0x14, 8);
        word(&mut d, at + 0x18, 8);
        word(&mut d, at + 0x30, 0);
        word(&mut d, at + 0x34, (image - at - 0x34) as u32);
        word(&mut d, image, if i == 1 { 0 } else { 8 });
        word(&mut d, image + 4, 8);
        word(&mut d, image + 8, 256);
        word(&mut d, image + 12, (raw - image - 12) as u32);
        for p in d[raw..raw + 256].as_chunks_mut::<4>().0 {
            p.copy_from_slice(&[255, 10, 40, 90]);
        }
    }
    d
}
pub fn bch() -> Vec<u8> {
    let mut d = vec![0; 0x300];
    d[..4].copy_from_slice(b"BCH\0");
    d[4] = 0x21;
    for (at, v) in [
        (8, 0x80),
        (12, 0x180),
        (16, 0x1c0),
        (20, 0x200),
        (0x20, 0x100),
        (0x24, 0x40),
        (0x28, 24),
        (0x2c, 256),
    ] {
        word(&mut d, at, v);
    }
    word(&mut d, 0x80 + 36, 0x50);
    word(&mut d, 0x80 + 40, 1);
    word(&mut d, 0x80 + 0x50, 0x80);
    word(&mut d, 0x100 + 4, 6);
    d[0x180..0x180 + 15].copy_from_slice(b"en_goldhorn_t01");
    for (i, (value, reg)) in [(8 << 16 | 8, 0x82), (0, 0x8e), (0, 0x85)]
        .into_iter()
        .enumerate()
    {
        word(&mut d, 0x1c0 + i * 8, value);
        word(&mut d, 0x1c4 + i * 8, 0xf0000 | reg);
    }
    for p in d[0x200..].as_chunks_mut::<4>().0 {
        p.copy_from_slice(&[128, 30, 60, 90]);
    }
    d
}
pub fn romfs(file: &str, payload: &[u8]) -> Vec<u8> {
    let name: Vec<_> = file.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let file_size = (32 + name.len()).div_ceil(4) * 4;
    let data_off = 0x80 + file_size;
    let mut d = vec![0; data_off + payload.len()];
    word(&mut d, 0, 0x28);
    word(&mut d, 12, 0x40);
    word(&mut d, 16, 24);
    word(&mut d, 28, 0x80);
    word(&mut d, 32, file_size as u32);
    word(&mut d, 36, data_off as u32);
    for at in [0x40, 0x44, 0x48, 0x50] {
        word(&mut d, at, u32::MAX);
    }
    word(&mut d, 0x4c, 0);
    word(&mut d, 0x84, u32::MAX);
    long(&mut d, 0x90, payload.len() as u64);
    word(&mut d, 0x9c, name.len() as u32);
    d[0xa0..0xa0 + name.len()].copy_from_slice(&name);
    d[data_off..].copy_from_slice(payload);
    d
}
pub fn ncch() -> Vec<u8> {
    let r = romfs("texture.stex", &stex("ig_bat_miss", 2, 0));
    let rs = (0x1000 + r.len()).div_ceil(512) * 512;
    let mut d = vec![0; 512 + rs];
    d[0x100..0x104].copy_from_slice(b"NCCH");
    word(&mut d, 0x1b0, 1);
    word(&mut d, 0x1b4, (rs / 512) as u32);
    long(&mut d, 0x118, 0x00040000000ec700);
    d[0x150..0x15a].copy_from_slice(b"CTR-P-BSKE");
    d[512..516].copy_from_slice(b"IVFC");
    word(&mut d, 512 + 0x4c, 12);
    d[512 + 0x1000..512 + 0x1000 + r.len()].copy_from_slice(&r);
    d
}
pub fn cia() -> Vec<u8> {
    let content = ncch();
    let tmd_size: usize = 0x140 + 0x9c4 + 48;
    let tmd_off = 0x2040;
    let content_off = (tmd_off + tmd_size).div_ceil(64) * 64;
    let mut d = vec![0; content_off + content.len()];
    word(&mut d, 0, 0x2020);
    word(&mut d, 16, tmd_size as u32);
    long(&mut d, 24, content.len() as u64);
    d[0x20] = 0x80;
    d[tmd_off..tmd_off + 4].copy_from_slice(&0x10004u32.to_be_bytes());
    d[tmd_off + 0x140 + 0x9e..tmd_off + 0x140 + 0xa0].copy_from_slice(&1u16.to_be_bytes());
    d[tmd_off + 0x140 + 0x9c4 + 8..tmd_off + 0x140 + 0x9c4 + 16]
        .copy_from_slice(&(content.len() as u64).to_be_bytes());
    d[content_off..].copy_from_slice(&content);
    d
}
