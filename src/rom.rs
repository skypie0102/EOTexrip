use crate::{archive, binary::*};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::{BTreeSet, VecDeque},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Resource {
    pub path: String,
    pub disk: PathBuf,
    pub offset: u64,
    pub size: u64,
}
impl Resource {
    pub fn read(&self) -> Result<Vec<u8>> {
        Reader::new(&self.disk)?.range(self.offset, self.size)
    }
    pub fn slice(&self, path: String, offset: u64, size: u64) -> Result<Self> {
        ensure!(
            offset.checked_add(size).is_some_and(|n| n <= self.size),
            "member outside source resource"
        );
        Ok(Self {
            path,
            disk: self.disk.clone(),
            offset: self
                .offset
                .checked_add(offset)
                .ok_or_else(|| anyhow::anyhow!("offset overflow"))?,
            size,
        })
    }
}
pub struct Input {
    pub resources: Vec<Resource>,
    pub title_id: Option<String>,
    pub product_code: Option<String>,
}
struct Reader {
    file: File,
    len: u64,
}
impl Reader {
    fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
    fn range(&mut self, offset: u64, size: u64) -> Result<Vec<u8>> {
        ensure!(
            size <= archive::MAX_RESOURCE as u64,
            "resource exceeds 256 MiB read limit"
        );
        ensure!(
            offset.checked_add(size).is_some_and(|end| end <= self.len),
            "range outside input file"
        );
        self.file.seek(SeekFrom::Start(offset))?;
        let mut out = vec![0; size as usize];
        self.file.read_exact(&mut out)?;
        Ok(out)
    }
}
fn dir_entry(data: &[u8], offset: usize) -> Result<(u32, u32, u32, String)> {
    let count = u32le(data, offset + 20)? as usize;
    ensure!(
        count <= 2048 && count.is_multiple_of(2),
        "invalid RomFS directory name"
    );
    let raw = bytes(data, offset + 24, count)?;
    let utf: Vec<_> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect();
    Ok((
        u32le(data, offset + 4)?,
        u32le(data, offset + 8)?,
        u32le(data, offset + 12)?,
        String::from_utf16(&utf)?,
    ))
}
fn romfs(reader: &mut Reader, disk: &Path, start: u64, size: u64) -> Result<Vec<Resource>> {
    ensure!(
        start.checked_add(size).is_some_and(|n| n <= reader.len),
        "RomFS range exceeds container"
    );
    let head = reader.range(start, size.min(0x60))?;
    let l3 = if head.starts_with(b"IVFC") {
        ensure!(head.len() >= 0x50, "short IVFC header");
        let exp = u32le(&head, 0x4c)?;
        ensure!(exp <= 20, "invalid IVFC block size");
        // IVFC physical layout places level 3 after the master hash, aligned to
        // the level-3 block size; logical offsets describe the hash hierarchy.
        align(0x60 + u32le(&head, 8)? as u64, 1u64 << exp)?
    } else if u32le(&head, 0)? == 0x28 {
        0
    } else {
        bail!("RomFS has no decrypted IVFC/level-3 header");
    };
    ensure!(l3 + 0x28 <= size, "RomFS level 3 outside container");
    let header = reader.range(start + l3, 0x28)?;
    ensure!(u32le(&header, 0)? == 0x28, "invalid RomFS level-3 header");
    let dir_off = u32le(&header, 12)? as u64;
    let dir_size = u32le(&header, 16)? as u64;
    let file_off = u32le(&header, 28)? as u64;
    let file_size = u32le(&header, 32)? as u64;
    let data_off = u32le(&header, 36)? as u64;
    for (off, len) in [(dir_off, dir_size), (file_off, file_size)] {
        ensure!(
            off >= 0x28 && l3 + off + len <= size,
            "RomFS metadata range invalid"
        );
    }
    ensure!(
        data_off >= file_off + file_size && l3 + data_off <= size,
        "invalid RomFS data offset"
    );
    let dirs = reader.range(start + l3 + dir_off, dir_size)?;
    let files = reader.range(start + l3 + file_off, file_size)?;
    let mut queue = VecDeque::from([(0u32, String::new())]);
    let mut seen_dirs = BTreeSet::new();
    let mut seen_files = BTreeSet::new();
    let mut out = vec![];
    while let Some((offset, parent)) = queue.pop_front() {
        ensure!(
            seen_dirs.insert(offset),
            "cycle or duplicate RomFS directory"
        );
        ensure!(seen_dirs.len() <= 100000, "RomFS directory-count limit");
        let (sibling, child, mut file, name) = dir_entry(&dirs, offset as usize)?;
        let path = if offset == 0 {
            String::new()
        } else {
            logical_path(format!("{parent}/{name}").trim_start_matches('/'))?
        };
        while file != u32::MAX {
            ensure!(seen_files.insert(file), "cycle or duplicate RomFS file");
            ensure!(seen_files.len() <= 100000, "RomFS file-count limit");
            let at = file as usize;
            let name_size = u32le(&files, at + 28)? as usize;
            ensure!(
                name_size <= 2048 && name_size.is_multiple_of(2),
                "invalid RomFS filename"
            );
            let raw = bytes(&files, at + 32, name_size)?;
            let utf: Vec<_> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|x| u16::from_le_bytes([x[0], x[1]]))
                .collect();
            let name = String::from_utf16(&utf)?;
            let logical = logical_path(format!("{path}/{name}").trim_start_matches('/'))?;
            let offset = u64le(&files, at + 8)?;
            let len = u64le(&files, at + 16)?;
            let relative = l3
                .checked_add(data_off)
                .and_then(|n| n.checked_add(offset))
                .ok_or_else(|| anyhow::anyhow!("RomFS file offset overflow"))?;
            ensure!(
                relative.checked_add(len).is_some_and(|n| n <= size),
                "RomFS file {logical} outside container"
            );
            out.push(Resource {
                path: logical,
                disk: disk.into(),
                offset: start + relative,
                size: len,
            });
            file = u32le(&files, at + 4)?;
        }
        if child != u32::MAX {
            queue.push_back((child, path));
        }
        if sibling != u32::MAX {
            queue.push_back((sibling, parent));
        }
    }
    Ok(out)
}
fn ncch(reader: &mut Reader, path: &Path, base: u64, len: u64) -> Result<Input> {
    let header = reader.range(base, 0x200)?;
    ensure!(bytes(&header, 0x100, 4)? == b"NCCH", "not NCCH");
    let off = u32le(&header, 0x1b0)? as u64 * 512;
    let size = u32le(&header, 0x1b4)? as u64 * 512;
    ensure!(
        off >= 512 && size > 0 && off.checked_add(size).is_some_and(|n| n <= len),
        "NCCH has no valid RomFS"
    );
    let magic = reader.range(base + off, 4)?;
    ensure!(
        magic == b"IVFC",
        "RomFS is encrypted or unsupported; select a decrypted game dump"
    );
    let program = u64le(&header, 0x118)?;
    let title = if program == 0 {
        u64le(&header, 0x108)?
    } else {
        program
    };
    let product = String::from_utf8_lossy(&header[0x150..0x160])
        .trim_end_matches('\0')
        .to_owned();
    Ok(Input {
        resources: romfs(reader, path, base + off, size)?,
        title_id: Some(format!("{title:016X}")),
        product_code: Some(product),
    })
}
pub fn open(path: &Path) -> Result<Input> {
    if path.is_dir() {
        let mut resources = vec![];
        for entry in walkdir::WalkDir::new(path).follow_links(false) {
            let entry = entry?;
            ensure!(
                !entry.file_type().is_symlink(),
                "input directories must not contain symbolic links"
            );
            if entry.file_type().is_file() {
                let relative = entry.path().strip_prefix(path)?.to_string_lossy();
                resources.push(Resource {
                    path: logical_path(&relative)?,
                    disk: entry.path().into(),
                    offset: 0,
                    size: entry.metadata()?.len(),
                });
                ensure!(resources.len() <= 100000, "input file-count limit");
            }
        }
        resources.sort_by(|a, b| a.path.cmp(&b.path));
        return Ok(Input {
            resources,
            title_id: None,
            product_code: None,
        });
    }
    let mut reader = Reader::new(path)?;
    let len = reader.len;
    let head = reader.range(0, len.min(0x200))?;
    if bytes(&head, 0x100, 4).ok() == Some(b"NCCH" as &[u8]) {
        return ncch(&mut reader, path, 0, len);
    }
    if bytes(&head, 0x100, 4).ok() == Some(b"NCSD" as &[u8]) {
        for i in 0..8 {
            let off = u32le(&head, 0x120 + i * 8)? as u64 * 512;
            let size = u32le(&head, 0x124 + i * 8)? as u64 * 512;
            if size == 0 {
                continue;
            }
            ensure!(
                off.checked_add(size).is_some_and(|n| n <= len),
                "NCSD partition outside file"
            );
            let h = reader.range(off, 512)?;
            if bytes(&h, 0x100, 4)? == b"NCCH" && u32le(&h, 0x1b4)? > 0 {
                return ncch(&mut reader, path, off, size);
            }
        }
        bail!("NCSD contains no supported game RomFS");
    }
    if head.starts_with(b"IVFC") || u32le(&head, 0).ok() == Some(0x28) {
        return Ok(Input {
            resources: romfs(&mut reader, path, 0, len)?,
            title_id: None,
            product_code: None,
        });
    }
    if u32le(&head, 0).ok() == Some(0x2020) {
        let header = reader.range(0, 0x2020)?;
        let cert = u32le(&header, 8)? as u64;
        let ticket = u32le(&header, 12)? as u64;
        let tmd_size = u32le(&header, 16)? as u64;
        let content_size = u64le(&header, 24)?;
        let tmd_off = align(align(align(0x2020, 64)? + cert, 64)? + ticket, 64)?;
        let content = align(tmd_off + tmd_size, 64)?;
        let tmd = reader.range(tmd_off, tmd_size)?;
        ensure!(
            content.checked_add(content_size).is_some_and(|n| n <= len),
            "CIA content outside file"
        );
        let sig = match u32be(&tmd, 0)? {
            0x10000 | 0x10003 => 0x240,
            0x10001 | 0x10004 => 0x140,
            0x10002 | 0x10005 => 0x80,
            _ => bail!("unsupported CIA TMD signature type"),
        };
        let count = u16be(&tmd, sig + 0x9e)? as usize;
        ensure!(count > 0 && count <= 65536, "invalid CIA content count");
        let mut position = content;
        for i in 0..count {
            let record = sig + 0x9c4 + i * 48;
            let index = u16be(&tmd, record + 4)? as usize;
            let size = u64be(&tmd, record + 8)?;
            // The header bitmap defines which TMD chunks are actually present.
            if header
                .get(0x20 + index / 8)
                .is_none_or(|b| b & (0x80 >> (index % 8)) == 0)
            {
                continue;
            }
            ensure!(
                position
                    .checked_add(size)
                    .is_some_and(|n| n <= content + content_size),
                "CIA chunk outside content range"
            );
            let h = reader.range(position, size.min(512))?;
            if bytes(&h, 0x100, 4).ok() == Some(b"NCCH" as &[u8]) && u32le(&h, 0x1b4)? > 0 {
                return ncch(&mut reader, path, position, size);
            }
            position = position
                .checked_add(size)
                .ok_or_else(|| anyhow::anyhow!("CIA offset overflow"))?;
        }
        bail!("CIA contains no decrypted supported game RomFS");
    }
    ensure!(
        len <= archive::MAX_RESOURCE as u64,
        "unrecognized input container"
    );
    Ok(Input {
        resources: vec![Resource {
            path: path
                .file_name()
                .context("input has no filename")?
                .to_string_lossy()
                .into_owned(),
            disk: path.into(),
            offset: 0,
            size: len,
        }],
        title_id: None,
        product_code: None,
    })
}
