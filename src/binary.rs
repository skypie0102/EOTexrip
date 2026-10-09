use anyhow::{Result, bail, ensure};

pub fn bytes(data: &[u8], offset: usize, size: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| anyhow::anyhow!("range overflow"))?;
    data.get(offset..end).ok_or_else(|| {
        anyhow::anyhow!(
            "range {offset:#x}..{end:#x} exceeds resource length {:#x}",
            data.len()
        )
    })
}
pub fn u16le(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes(data, offset, 2)?.try_into()?))
}
pub fn u32le(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes(data, offset, 4)?.try_into()?))
}
pub fn u64le(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes(data, offset, 8)?.try_into()?))
}
pub fn u16be(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(bytes(data, offset, 2)?.try_into()?))
}
pub fn u32be(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(bytes(data, offset, 4)?.try_into()?))
}
pub fn u64be(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_be_bytes(bytes(data, offset, 8)?.try_into()?))
}
pub fn relative(data: &[u8], field: usize) -> Result<usize> {
    let delta = u32le(data, field)? as i32 as i64;
    let value = (field as i64)
        .checked_add(delta)
        .ok_or_else(|| anyhow::anyhow!("relative pointer overflow"))?;
    ensure!(
        value >= 0 && value < data.len() as i64,
        "relative pointer outside resource"
    );
    Ok(value as usize)
}
pub fn cstring(data: &[u8], offset: usize, limit: usize) -> Result<String> {
    ensure!(offset < data.len(), "string outside resource");
    let tail = &data[offset..data.len().min(offset.saturating_add(limit))];
    let end = tail
        .iter()
        .position(|&x| x == 0)
        .ok_or_else(|| anyhow::anyhow!("unterminated string"))?;
    Ok(std::str::from_utf8(&tail[..end])?.to_owned())
}
pub fn logical_path(value: &str) -> Result<String> {
    let value = value.replace('\\', "/");
    ensure!(
        !value.starts_with('/') && !value.contains(':') && !value.contains('\0'),
        "unsafe resource path"
    );
    let mut parts = Vec::new();
    for part in value.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            bail!("resource path escapes its container");
        }
        parts.push(part);
    }
    ensure!(!parts.is_empty(), "empty resource path");
    Ok(parts.join("/"))
}
pub fn align(value: u64, alignment: u64) -> Result<u64> {
    ensure!(alignment.is_power_of_two(), "invalid alignment");
    Ok(value
        .checked_add(alignment - 1)
        .ok_or_else(|| anyhow::anyhow!("alignment overflow"))?
        & !(alignment - 1))
}
