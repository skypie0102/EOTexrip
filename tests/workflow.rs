mod common;
use common::*;
use eotexrip::{
    catalog::{Category, Game, Override, Overrides},
    pipeline::{self, Options},
    workspace,
};
use std::{fs, path::Path, sync::atomic::AtomicBool};
const TITLE: &str = "00040000000EC700";
fn extract(input: &Path, output: &Path) -> anyhow::Result<eotexrip::catalog::Catalog> {
    pipeline::extract(
        &Options {
            input: input.into(),
            output: output.into(),
            game: Game::Eou,
            title_id: Some(TITLE.into()),
        },
        &AtomicBool::new(false),
        |_| {},
    )
}
fn input(root: &Path) {
    fs::create_dir_all(root).unwrap();
    let (index, payload) = hpi(&[
        (
            "STEX/KEYBOARD/KATAKANA/IG_KEY_KATAKANA045.STEX",
            stex("", 7, 0),
        ),
        ("STEX/FACILITY/BG/IG_FAC_BG_05.STEX", stex("", 8, 64)),
        ("EFFECT/ENEMY/SPELL.EPL/P_SMOKE.STEX", stex("p_smoke", 9, 0)),
        ("MONSTER/MODEL/EN001.BAM", cgfx(false)),
    ]);
    fs::write(root.join("MORI1R.HPI"), index).unwrap();
    fs::write(root.join("MORI1R.HPB"), payload).unwrap();
}
#[test]
fn unavailable_untold_members_do_not_block_valid_textures_in_any_namespace() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    fs::create_dir_all(&i).unwrap();
    let (mut index, payload) = hpi(&[
        (
            "STEX/KEYBOARD/KATAKANA/IG_KEY_KATAKANA045.STEX",
            stex("", 7, 0),
        ),
        ("SAVEDATA/MO1R04_GAME.SAV", vec![]),
        ("SND/MORI1R/STREAM/NBGM_BATTLE2.DSPADPCM.BCSTM", vec![]),
        ("OTHER/UNAVAILABLE.BIN", vec![]),
        ("STEX/TRUNCATED.STEX", vec![]),
        ("STEX/FACILITY/BG/IG_FAC_BG_05.STEX", stex("", 8, 0)),
        // Even a sound payload that resembles a texture must be excluded.
        (
            "SND/MORI1R/STREAM/BOUNDED.Bcstm",
            stex("not_a_texture", 9, 0),
        ),
        ("SAVEDATA/BOUNDED.SAV", stex("not_a_texture", 10, 0)),
    ]);
    for entry in [1, 2] {
        word(&mut index, 24 + entry * 16 + 4, u32::MAX);
        word(&mut index, 24 + entry * 16 + 8, 4096);
    }
    word(&mut index, 24 + 3 * 16 + 4, payload.len() as u32 + 1);
    word(&mut index, 24 + 4 * 16 + 4, payload.len() as u32 - 1);
    word(&mut index, 24 + 4 * 16 + 8, 2);
    fs::write(i.join("MORI1R.HPI"), index).unwrap();
    fs::write(i.join("MORI1R.HPB"), payload).unwrap();
    let c = extract(&i, &o).unwrap();
    assert_eq!(c.summary.unique_images, 2);
    assert_eq!(c.summary.resources, 2);
    assert_eq!(c.issues.len(), 4);
    assert!(
        c.issues
            .iter()
            .all(|issue| issue.stage == "archive_member_bounds")
    );
    for name in [
        "SAVEDATA/MO1R04_GAME.SAV",
        "SND/MORI1R/STREAM/NBGM_BATTLE2.DSPADPCM.BCSTM",
        "OTHER/UNAVAILABLE.BIN",
        "STEX/TRUNCATED.STEX",
    ] {
        assert!(
            c.issues
                .iter()
                .any(|issue| issue.source.ends_with(name) && issue.message.contains("outside HPB"))
        );
    }
    assert!(c.assets.iter().all(|a| !a.source.contains("SAVEDATA")));
    assert!(c.assets.iter().all(|a| !a.source.contains("SND/")));
    assert!(c.assets.iter().all(|a| a.internal_name != "not_a_texture"));
    let report = workspace::load(&o).unwrap();
    assert_eq!(report.summary.issues, 4);
    assert_eq!(report.issues.len(), c.issues.len());
}
#[test]
fn unusable_hpi_input_preserves_existing_outputs() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    let c = extract(&i, &o).unwrap();
    let before_pack = fs::read(o.join("azahar_pack_master/pack.json")).unwrap();
    let a = &c.assets[0];
    let master = o.join("azahar_pack_master").join(&a.master_file);
    let before_master = fs::read(&master).unwrap();
    for name in [
        "STEX/BAD.STEX",
        "SAVEDATA/BAD.STEX",
        "OTHER/BAD.SAV",
        "SAVEDATA/../BAD.SAV",
    ] {
        let (mut index, payload) = hpi(&[(name, stex("", 9, 0))]);
        word(&mut index, 28, u32::MAX);
        fs::write(i.join("MORI1R.HPI"), index).unwrap();
        fs::write(i.join("MORI1R.HPB"), payload).unwrap();
        assert!(extract(&i, &o).is_err(), "must reject {name}");
        assert_eq!(fs::read(&master).unwrap(), before_master);
        assert_eq!(
            fs::read(o.join("azahar_pack_master/pack.json")).unwrap(),
            before_pack
        );
    }
}
#[test]
fn partial_archive_retains_edited_master_for_unavailable_texture() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    let first = extract(&i, &o).unwrap();
    let missing = first
        .assets
        .iter()
        .find(|a| a.category.category == Category::Backgrounds)
        .unwrap();
    let path = o.join("azahar_pack_master").join(&missing.master_file);
    let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), 16, 16);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[123; 16 * 16 * 4])
        .unwrap();
    let edited = fs::read(&path).unwrap();
    let pack_before = fs::read(o.join("azahar_pack_master/pack.json")).unwrap();
    let index_path = i.join("MORI1R.HPI");
    let mut index = fs::read(&index_path).unwrap();
    word(&mut index, 24 + 16 + 4, u32::MAX);
    fs::write(index_path, index).unwrap();
    let partial = extract(&i, &o).unwrap();
    assert_eq!(partial.summary.textures, 3);
    assert_eq!(partial.summary.unique_images, 4);
    assert!(
        partial
            .issues
            .iter()
            .any(|issue| issue.stage == "archive_member_bounds" && issue.source == missing.source)
    );
    assert!(
        partial
            .issues
            .iter()
            .any(|issue| issue.stage == "preservation" && issue.source == missing.source)
    );
    assert_eq!(fs::read(path).unwrap(), edited);
    assert_eq!(
        fs::read(o.join(format!("azahar_pack/{TITLE}/{}", missing.master_file))).unwrap(),
        edited
    );
    assert_eq!(
        fs::read(o.join("azahar_pack_master/pack.json")).unwrap(),
        pack_before
    );
}
#[test]
fn oversized_and_bad_compressed_members_are_reported_without_blocking_others() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    fs::create_dir_all(&i).unwrap();
    let mut packed = b"ACMPxx".to_vec();
    packed.extend_from_slice(&[0, 0xf0, b'a', b'b', b'c', 0x10]);
    packed.extend_from_slice(&(8u32 << 24 | 14).to_le_bytes());
    packed.extend_from_slice(&7u32.to_le_bytes());
    let mut nested = vec![0; 32];
    nested[..4].copy_from_slice(b"ACMP");
    word(&mut nested, 4, packed.len() as u32);
    word(&mut nested, 8, 32);
    word(&mut nested, 16, 27);
    nested.extend_from_slice(&packed);
    assert!(
        eotexrip::archive::acmp(&nested)
            .unwrap()
            .starts_with(b"ACMP")
    );
    let mut mismatch = nested.clone();
    mismatch[32..38].copy_from_slice(b"PREFIX");
    let (mut index, payload) = hpi(&[
        ("STEX/VALID_BEFORE.STEX", stex("", 7, 0)),
        ("STEX/TOO_LARGE.STEX", stex("", 8, 0)),
        ("STEX/BAD_COMPRESSED.STEX", b"ACMP".to_vec()),
        ("SND/MORI1R/STREAM/BOUNDED.BCSTM", b"ACMP".to_vec()),
        ("STEX/NESTED_BAD.STEX", nested),
        ("STEX/SIZE_MISMATCH.STEX", mismatch),
        ("STEX/VALID_AFTER.STEX", stex("", 10, 0)),
    ]);
    word(&mut index, 24 + 5 * 16 + 12, 28);
    word(
        &mut index,
        24 + 16 + 12,
        eotexrip::archive::MAX_RESOURCE as u32 + 1,
    );
    fs::write(i.join("MORI1R.HPI"), &index).unwrap();
    fs::write(i.join("MORI1R.HPB"), payload).unwrap();
    let c = extract(&i, &o).unwrap();
    assert_eq!(c.summary.unique_images, 2);
    assert!(
        c.issues.iter().any(
            |issue| issue.stage == "resource_limit" && issue.source.ends_with("TOO_LARGE.STEX")
        )
    );
    assert!(
        c.issues
            .iter()
            .any(|issue| issue.stage == "archive_member_decode"
                && issue.source.ends_with("BAD_COMPRESSED.STEX"))
    );
    assert!(
        c.issues.iter().any(
            |issue| issue.stage == "decompression" && issue.source.ends_with("NESTED_BAD.STEX")
        )
    );
    assert!(
        c.issues
            .iter()
            .any(|issue| issue.stage == "archive_member_decode"
                && issue.source.ends_with("SIZE_MISMATCH.STEX")
                && issue.message.contains("expansion size mismatch"))
    );
    assert_eq!(c.issues.len(), 4);
    // A stored range can be valid in a large HPB yet exceed the decode budget.
    word(&mut index, 24 + 16 + 4, 0);
    word(
        &mut index,
        24 + 16 + 8,
        eotexrip::archive::MAX_RESOURCE as u32 + 1,
    );
    word(&mut index, 24 + 16 + 12, 0);
    let parsed =
        eotexrip::archive::hpi(&index, eotexrip::archive::MAX_RESOURCE as u64 + 1).unwrap();
    assert_eq!(parsed.issues.len(), 1);
    assert_eq!(parsed.issues[0].stage, "resource_limit");
    assert!(
        !parsed
            .members
            .iter()
            .any(|m| m.name.ends_with("TOO_LARGE.STEX"))
    );
}
#[test]
fn damaged_hpi_structure_remains_fatal_even_with_valid_texture_members() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    extract(&i, &o).unwrap();
    let before = fs::read(o.join("azahar_pack_master/pack.json")).unwrap();
    for name in ["SAVEDATA/../BAD.SAV", "C:/BAD.STEX"] {
        let (index, payload) = hpi(&[("STEX/VALID.STEX", stex("", 7, 0)), (name, vec![])]);
        fs::write(i.join("MORI1R.HPI"), index).unwrap();
        fs::write(i.join("MORI1R.HPB"), payload).unwrap();
        assert!(extract(&i, &o).is_err());
        assert_eq!(
            fs::read(o.join("azahar_pack_master/pack.json")).unwrap(),
            before
        );
    }
    let (index, payload) = hpi(&[
        ("STEX/VALID.STEX", stex("", 7, 0)),
        ("STEX/OTHER.STEX", vec![]),
    ]);
    for corrupted in [index[..39].to_vec(), {
        let mut data = index.clone();
        word(&mut data, 24 + 16, u32::MAX);
        data
    }] {
        fs::write(i.join("MORI1R.HPI"), corrupted).unwrap();
        fs::write(i.join("MORI1R.HPB"), &payload).unwrap();
        assert!(extract(&i, &o).is_err());
        assert_eq!(
            fs::read(o.join("azahar_pack_master/pack.json")).unwrap(),
            before
        );
    }
}
#[test]
fn real_archive_to_flat_pack_preserves_hash_span_and_orientation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    input(&root.join("in"));
    let c = extract(&root.join("in"), &root.join("out")).unwrap();
    assert_eq!(c.summary.textures, 4);
    assert_eq!(c.summary.unique_images, 4);
    for category in [
        Category::Fonts,
        Category::Backgrounds,
        Category::Effects,
        Category::Monsters,
    ] {
        assert!(c.assets.iter().any(|a| a.category.category == category));
    }
    let font = c
        .assets
        .iter()
        .find(|a| a.category.category == Category::Fonts)
        .unwrap();
    let path = root.join("out/azahar_pack_master").join(&font.master_file);
    let mut r = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).unwrap()))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; r.output_buffer_size().unwrap()];
    r.next_frame(&mut pixels).unwrap();
    assert_eq!(&pixels[..4], &[0, 20, 7, 255]);
    assert_eq!(&pixels[7 * 8 * 4..7 * 8 * 4 + 4], &[42, 20, 7, 255]);
    let bg = c
        .assets
        .iter()
        .find(|a| a.category.category == Category::Backgrounds)
        .unwrap();
    assert_eq!(
        bg.runtime_hashes[0],
        eotexrip::pica::runtime_hash(&stex("", 8, 64)[128..384])
    );
    let pack: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join(format!("out/azahar_pack/{TITLE}/pack.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(pack["options"]["use_new_hash"], true);
    // Azahar's GPU decode reverses rows; its PNG loader performs that
    // reversal once. Editable and deployed PNGs both keep artwork order.
    assert_eq!(pack["options"]["flip_png_files"], true);
    assert_eq!(pack["textures"].as_object().unwrap().len(), 4);
    assert!(!root.join("out/.eouhd/journal.json").exists());
    assert!(
        !root
            .join("out/azahar_pack_master/pack.json.previous")
            .exists()
    );
}
#[test]
fn edited_upscaled_master_and_confirmed_choices_survive_rerun() {
    let t = tempfile::tempdir().unwrap();
    let input_dir = t.path().join("in");
    let out = t.path().join("out");
    input(&input_dir);
    let first = extract(&input_dir, &out).unwrap();
    let a = &first.assets[0];
    let oldpath = out.join("azahar_pack_master").join(&a.master_file);
    let mut image = png::Encoder::new(fs::File::create(&oldpath).unwrap(), 16, 16);
    image.set_color(png::ColorType::Rgba);
    image.set_depth(png::BitDepth::Eight);
    image
        .write_header()
        .unwrap()
        .write_image_data(&[123; 16 * 16 * 4])
        .unwrap();
    let edited = fs::read(&oldpath).unwrap();
    let mut choices = Overrides::default();
    choices.assets.insert(
        a.id.clone(),
        Override {
            category: Some(Category::Ui),
            name: Some("my readable texture".into()),
            confirmed: true,
            png_orientation: None,
        },
    );
    let changed = workspace::reassign(&out, choices).unwrap();
    let changed_a = changed.assets.iter().find(|n| n.id == a.id).unwrap();
    assert_eq!(changed_a.master_file, "ui/my_readable_texture.png");
    assert_eq!(
        fs::read(out.join("azahar_pack_master").join(&changed_a.master_file)).unwrap(),
        edited
    );
    let rerun = extract(&input_dir, &out).unwrap();
    let retained = rerun.assets.iter().find(|n| n.id == a.id).unwrap();
    assert_eq!(retained.master_file, changed_a.master_file);
    assert_eq!(
        fs::read(out.join("azahar_pack_master").join(&retained.master_file)).unwrap(),
        edited
    );
    assert_eq!(
        fs::read(out.join(format!("azahar_pack/{TITLE}/{}", retained.master_file))).unwrap(),
        edited
    );
}
#[test]
fn live_pack_manual_rename_becomes_persistent_choice() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    let first = extract(&i, &o).unwrap();
    let a = &first.assets[0];
    let masters = o.join("azahar_pack_master");
    fs::create_dir_all(masters.join("icons")).unwrap();
    fs::rename(
        masters.join(&a.master_file),
        masters.join("icons/user_name.png"),
    )
    .unwrap();
    let path = masters.join("pack.json");
    let mut pack: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for h in &a.runtime_hashes {
        pack["textures"][h] = "user_name.png".into();
    }
    fs::write(path, serde_json::to_vec(&pack).unwrap()).unwrap();
    let updated = extract(&i, &o).unwrap();
    let a2 = updated.assets.iter().find(|n| n.id == a.id).unwrap();
    assert_eq!(a2.master_file, "icons/user_name.png");
    let again = extract(&i, &o).unwrap();
    assert_eq!(
        again
            .assets
            .iter()
            .find(|n| n.id == a.id)
            .unwrap()
            .master_file,
        "icons/user_name.png"
    );
}
#[test]
fn failed_input_cancel_and_bad_catalog_preserve_existing_pack() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    let c = extract(&i, &o).unwrap();
    let before = fs::read(o.join("azahar_pack_master/pack.json")).unwrap();
    fs::write(i.join("MORI1R.HPI"), b"broken").unwrap();
    assert!(extract(&i, &o).is_err());
    assert_eq!(
        fs::read(o.join("azahar_pack_master/pack.json")).unwrap(),
        before
    );
    assert!(
        pipeline::extract(
            &Options {
                input: i,
                output: o.clone(),
                game: Game::Eou,
                title_id: Some(TITLE.into())
            },
            &AtomicBool::new(true),
            |_| {}
        )
        .is_err()
    );
    fs::write(o.join(".eouhd/catalog.json"), b"corrupt").unwrap();
    assert!(workspace::Workspace::open(&o, Game::Eou).is_err());
    for a in c.assets {
        assert!(o.join("azahar_pack_master").join(a.master_file).is_file());
    }
}
#[test]
fn dedup_retains_multiple_hashes_and_provenance() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    fs::create_dir_all(i.join("FONT")).unwrap();
    let mut rgba = stex("one", 0, 0);
    for p in rgba[128..].as_chunks_mut::<4>().0 {
        p.copy_from_slice(&[255, 0, 0, 0]);
    }
    let mut rgb = stex("two", 0, 0);
    word(&mut rgb, 20, 0x8363);
    word(&mut rgb, 24, 0x6754);
    word(&mut rgb, 28, 128);
    rgb.truncate(256);
    rgb[128..].fill(0);
    fs::write(i.join("FONT/A.STEX"), rgba).unwrap();
    fs::write(i.join("FONT/B.STEX"), rgb).unwrap();
    let c = extract(&i, &o).unwrap();
    assert_eq!(c.summary.textures, 2);
    assert_eq!(c.summary.unique_images, 1);
    assert_eq!(c.assets[0].runtime_hashes.len(), 2);
    assert_eq!(c.assets[0].origins.len(), 2);
    assert_eq!(c.assets[0].aliases.len(), 1);
}
#[test]
fn typed_model_parsers_and_partial_failure_are_accounted() {
    let (good, issues) = eotexrip::resources::scan(&cgfx(true));
    assert_eq!(good.len(), 1);
    assert_eq!(issues.len(), 1);
    assert_eq!(good[0].name, "en_goldhorn_t00");
    let (textures, issues) = eotexrip::resources::scan(&bch());
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(textures.len(), 1);
    assert_eq!(textures[0].width, 8);
    assert_eq!(textures[0].name, "en_goldhorn_t01");
}
#[test]
fn ncch_cia_and_ncsd_ranges_reach_the_same_romfs() {
    let t = tempfile::tempdir().unwrap();
    let cxi = t.path().join("test.cxi");
    fs::write(&cxi, ncch()).unwrap();
    let cia_file = t.path().join("test.cia");
    fs::write(&cia_file, cia()).unwrap();
    let mut cart = vec![0; 512];
    cart[0x100..0x104].copy_from_slice(b"NCSD");
    word(&mut cart, 0x120, 1);
    word(&mut cart, 0x124, (ncch().len() / 512) as u32);
    cart.extend(ncch());
    let cart_file = t.path().join("test.3ds");
    fs::write(&cart_file, cart).unwrap();
    for path in [&cxi, &cia_file, &cart_file] {
        let input = eotexrip::rom::open(path).unwrap();
        assert_eq!(input.title_id.as_deref(), Some(TITLE));
        assert_eq!(input.resources.len(), 1);
        assert!(input.resources[0].read().unwrap().starts_with(b"STEX"));
    }
    let mut encrypted = ncch();
    encrypted[512] = 0;
    fs::write(&cxi, encrypted).unwrap();
    assert!(eotexrip::rom::open(&cxi).is_err());
}
#[test]
fn duplicate_names_are_stable_and_no_master_wins_by_input_order() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    fs::create_dir_all(i.join("FONT")).unwrap();
    for (name, seed) in [("A", 1), ("B", 2)] {
        fs::write(
            i.join(format!("FONT/{name}.STEX")),
            stex("duplicate", seed, 0),
        )
        .unwrap();
    }
    let c = extract(&i, &o).unwrap();
    assert_ne!(c.assets[0].name, c.assets[1].name);
    assert!(c.assets.iter().all(|a| a.name.starts_with("duplicate_")));
    let again = extract(&i, &o).unwrap();
    assert_eq!(
        c.assets.iter().map(|a| &a.name).collect::<Vec<_>>(),
        again.assets.iter().map(|a| &a.name).collect::<Vec<_>>()
    );
}
#[cfg(unix)]
#[test]
fn symbolic_link_managed_paths_are_rejected() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out");
    fs::create_dir_all(out.join(".eouhd")).unwrap();
    std::os::unix::fs::symlink(t.path(), out.join(".eouhd/jobs")).unwrap();
    assert!(workspace::Workspace::open(&out, Game::Eou).is_err());
}
