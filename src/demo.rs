use anyhow::{Result, ensure};
use std::{fs, path::Path};
/// Synthetic images only; no game assets are shipped with the application.
pub fn create(root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    ensure!(
        !root.join("DEMO.HPI").exists() && !root.join("DEMO.HPB").exists(),
        "demo files already exist in this folder"
    );
    let entries = [
        ("STEX/CHARAMAKE/CHARA/IG_CHA01.STEX", "ig_cha01"),
        ("MONSTER/DEMO.STEX", "demo_enemy"),
        ("STEX/WINDOW/IG_WINDOW.STEX", "ig_window"),
        ("STEX/ICON/IG_ICON.STEX", "ig_icon"),
        ("STEX/MAP/IG_DMAP.STEX", "ig_dmap"),
        ("DUNGEON/DEMO.STEX", "bf01_demo"),
        ("STEX/BG/IG_BG.STEX", "ig_bg"),
        ("EFFECT/DEMO.EPL/P_SMOKE.STEX", "p_smoke"),
        (
            "STEX/KEYBOARD/KATAKANA/IG_KEY_KATAKANA01.STEX",
            "ig_key_katakana01",
        ),
        ("UNKNOWN/DEMO.STEX", "demo_unknown"),
    ];
    let mut index = vec![0u8; 24 + entries.len() * 16];
    index[..4].copy_from_slice(b"HPIH");
    index[20..22].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    let mut names = vec![];
    let mut payload = vec![];
    for (i, (source, name)) in entries.into_iter().enumerate() {
        let mut image = vec![0; 128 + 64 * 64 * 4];
        image[..4].copy_from_slice(b"STEX");
        for (at, value) in [
            (12, 64),
            (16, 64),
            (20, 0x1401),
            (24, 0x6752),
            (28, 64 * 64 * 4),
            (32, 128),
        ] {
            image[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
        }
        image[40..40 + name.len()].copy_from_slice(name.as_bytes());
        for (p, pixel) in image[128..].as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let tile = p / 64;
            let dark = (tile / 8 + tile % 8) % 2 == 0;
            pixel.copy_from_slice(&[
                if p % 64 < 4 { 96 } else { 255 },
                if dark { 40 } else { 170 },
                (i as u8) * 21,
                if dark { 50 } else { 220 },
            ]);
        }
        let at = 24 + i * 16;
        for (field, value) in [(0, names.len()), (4, payload.len()), (8, image.len())] {
            index[at + field..at + field + 4].copy_from_slice(&(value as u32).to_le_bytes());
        }
        names.extend_from_slice(source.as_bytes());
        names.push(0);
        payload.extend(image);
    }
    index.extend(names);
    fs::write(root.join("DEMO.HPI"), index)?;
    fs::write(root.join("DEMO.HPB"), payload)?;
    Ok(())
}
