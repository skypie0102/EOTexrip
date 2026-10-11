mod common;
use common::*;
use eotexrip::{
    catalog::{Catalog, Game, Override, Overrides, PngOrientation},
    pipeline::{self, Options},
    png_image::Image,
    workspace,
};
use std::{collections::BTreeMap, fs, io::BufReader, path::Path, sync::atomic::AtomicBool};

const TITLE: &str = "000400000015D700";

fn extract(input: &Path, output: &Path) -> Catalog {
    pipeline::extract(
        &Options {
            input: input.into(),
            output: output.into(),
            game: Game::Eou2,
            title_id: Some(TITLE.into()),
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap()
}
fn input(root: &Path) {
    fs::create_dir_all(root).unwrap();
    let (index, payload) = hpi(&[
        ("STEX/KEYBOARD/IG_KEY_KATAKANA045.STEX", stex("", 7, 0)),
        ("STEX/FACILITY/BG/IG_FAC_BG_05.STEX", stex("", 8, 0)),
    ]);
    fs::write(root.join("MORI2R.HPI"), index).unwrap();
    fs::write(root.join("MORI2R.HPB"), payload).unwrap();
}
fn pixels(path: &Path) -> Vec<u8> {
    let mut reader = png::Decoder::new(BufReader::new(fs::File::open(path).unwrap()))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut bytes).unwrap();
    bytes.truncate(frame.buffer_size());
    bytes
}
fn export(output: &Path) -> Catalog {
    pipeline::export(output, &AtomicBool::new(false)).unwrap()
}
fn master(output: &Path, c: &Catalog, id: &str) -> std::path::PathBuf {
    output
        .join("azahar_pack_master")
        .join(&c.assets.iter().find(|a| a.id == id).unwrap().master_file)
}
fn deployment(output: &Path, c: &Catalog, id: &str) -> std::path::PathBuf {
    output
        .join("azahar_pack")
        .join(TITLE)
        .join(&c.assets.iter().find(|a| a.id == id).unwrap().master_file)
}
fn legacy_fixture(output: &Path, c: Catalog) -> (Catalog, BTreeMap<String, Vec<u8>>) {
    let mut c = c;
    let mut originals = BTreeMap::new();
    for a in &mut c.assets {
        let path = output.join("azahar_pack_master").join(&a.master_file);
        originals.insert(a.id.clone(), pixels(&path));
        let mut image = Image::read(&path).unwrap();
        image.flip_vertical();
        image.write(&path).unwrap();
        a.image_digest = image.rgba8_digest(a.width, a.height).unwrap();
    }
    let mut json = serde_json::to_value(&c).unwrap();
    json["version"] = "0.1.0-alpha.1".into();
    for a in json["assets"].as_array_mut().unwrap() {
        a.as_object_mut().unwrap().remove("png_orientation");
    }
    fs::write(
        output.join(".eouhd/catalog.json"),
        serde_json::to_vec_pretty(&json).unwrap(),
    )
    .unwrap();
    let old: Catalog = serde_json::from_value(json).unwrap();
    assert!(
        old.assets
            .iter()
            .all(|a| a.png_orientation == PngOrientation::LegacyFlipped)
    );
    (old, originals)
}
fn assert_repaired(
    output: &Path,
    old: &Catalog,
    repaired: &Catalog,
    originals: &BTreeMap<String, Vec<u8>>,
) {
    for a in &old.assets {
        let new = repaired.assets.iter().find(|n| n.id == a.id).unwrap();
        assert_eq!(new.png_orientation, PngOrientation::Upright);
        assert_eq!(new.master_file, a.master_file);
        assert_eq!(new.runtime_hashes, a.runtime_hashes);
        assert_eq!(new.category.category, a.category.category);
        assert_eq!(pixels(&master(output, repaired, &a.id)), originals[&a.id]);
        assert_eq!(
            fs::read(master(output, repaired, &a.id)).unwrap(),
            fs::read(deployment(output, repaired, &a.id)).unwrap()
        );
    }
    assert!(!repaired.issues.iter().any(|i| i.stage == "png_orientation"));
    assert_eq!(repaired.version, eotexrip::VERSION);
}

#[test]
fn old_untold2_originals_upgrade_once_on_export_or_reextraction() {
    for reextract in [false, true] {
        let t = tempfile::tempdir().unwrap();
        let i = t.path().join("in");
        let o = t.path().join("out");
        input(&i);
        let initial = extract(&i, &o);
        let (old, originals) = legacy_fixture(&o, initial);
        let upgraded = if reextract {
            extract(&i, &o)
        } else {
            export(&o)
        };
        assert_repaired(&o, &old, &upgraded, &originals);
        let bytes: Vec<_> = upgraded
            .assets
            .iter()
            .map(|a| fs::read(master(&o, &upgraded, &a.id)).unwrap())
            .collect();
        let again = export(&o);
        assert_repaired(&o, &old, &again, &originals);
        for (a, before) in again.assets.iter().zip(bytes) {
            assert_eq!(fs::read(master(&o, &again, &a.id)).unwrap(), before);
        }
    }
}

#[test]
fn already_manually_flipped_original_is_not_flipped_again() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    let initial = extract(&i, &o);
    let (old, originals) = legacy_fixture(&o, initial);
    let id = &old.assets[0].id;
    let path = master(&o, &old, id);
    let mut image = Image::read(&path).unwrap();
    image.flip_vertical();
    image.write(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let upgraded = export(&o);
    assert_repaired(&o, &old, &upgraded, &originals);
    assert_eq!(fs::read(master(&o, &upgraded, id)).unwrap(), before);
}

#[test]
fn edited_16_bit_upscale_keeps_master_bytes_and_normalizes_only_deployment() {
    let t = tempfile::tempdir().unwrap();
    let i = t.path().join("in");
    let o = t.path().join("out");
    input(&i);
    let initial = extract(&i, &o);
    let mut naming_choice = Overrides::default();
    naming_choice.assets.insert(
        initial.assets[0].id.clone(),
        Override {
            category: Some(eotexrip::catalog::Category::Ui),
            name: Some("my_saved_name".into()),
            confirmed: true,
            ..Override::default()
        },
    );
    let renamed = workspace::reassign(&o, naming_choice).unwrap();
    let (old, _) = legacy_fixture(&o, renamed);
    let id = &old.assets[0].id;
    let path = master(&o, &old, id);
    let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), 16, 16);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Sixteen);
    let samples: Vec<u8> = (0..16)
        .flat_map(|y| [y as u8, 45, 1, 2, 3, 4, 255, 128].repeat(16))
        .collect();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&samples)
        .unwrap();
    let edited = fs::read(&path).unwrap();
    let expected: Vec<u8> = samples
        .as_chunks::<{ 16 * 8 }>()
        .0
        .iter()
        .rev()
        .flatten()
        .copied()
        .collect();
    for _ in 0..2 {
        let c = export(&o);
        assert_eq!(
            c.assets
                .iter()
                .find(|a| &a.id == id)
                .unwrap()
                .png_orientation,
            PngOrientation::LegacyFlipped
        );
        assert_eq!(
            c.issues
                .iter()
                .filter(|i| i.stage == "png_orientation")
                .count(),
            1
        );
        assert_eq!(fs::read(master(&o, &c, id)).unwrap(), edited);
        assert_eq!(pixels(&deployment(&o, &c, id)), expected);
    }
    // After manually fixing the editable master, confirmation changes only
    // its interpretation. Saving and re-extracting must preserve those bytes.
    let mut image = Image::read(&path).unwrap();
    image.flip_vertical();
    image.write(&path).unwrap();
    let upright_edit = fs::read(&path).unwrap();
    let mut choices = Overrides::default();
    choices.assets.insert(
        id.clone(),
        Override {
            png_orientation: Some(PngOrientation::Upright),
            confirmed: true,
            ..Override::default()
        },
    );
    let confirmed = workspace::reassign(&o, choices).unwrap();
    assert_eq!(
        confirmed
            .assets
            .iter()
            .find(|a| &a.id == id)
            .unwrap()
            .master_file,
        old.assets[0].master_file
    );
    assert_eq!(fs::read(master(&o, &confirmed, id)).unwrap(), upright_edit);
    assert_eq!(
        fs::read(deployment(&o, &confirmed, id)).unwrap(),
        upright_edit
    );
    assert!(
        !confirmed
            .issues
            .iter()
            .any(|i| i.stage == "png_orientation")
    );
    let rerun = extract(&i, &o);
    assert_eq!(fs::read(master(&o, &rerun, id)).unwrap(), upright_edit);
    assert_eq!(fs::read(deployment(&o, &rerun, id)).unwrap(), upright_edit);
}
