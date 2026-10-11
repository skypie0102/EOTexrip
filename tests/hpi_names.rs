mod common;
use common::*;
use eotexrip::{
    archive,
    catalog::{Category, Game, Override, Overrides, PngOrientation},
    pipeline::{self, Options},
    workspace,
};
use std::{fs, path::Path, sync::atomic::AtomicBool};

const TITLE: &str = "00040000000EC700";
// Independent CP932 byte vectors, not produced by the decoder's encoder.
const NAMES: &[(&[u8], &str)] = &[
    (b"STEX/ICON/IG_ICON.STEX", "STEX/ICON/IG_ICON.STEX"),
    (b"STEX/BG/\x93\xfa\x96\x7b.STEX", "STEX/BG/日本.STEX"),
    (b"STEX\\\x95\x5c\\\x83\x5c.STEX", "STEX/表/ソ.STEX"),
    (
        b"STEX/\xee\xe0\x87\x40/\xb6\xc0\xb6\xc5.STEX",
        "STEX/髙①/ｶﾀｶﾅ.STEX",
    ),
    // These bytes are also valid UTF-8 (¥), but the HPI meaning is CP932.
    (b"STEX/\xc2\xa5.STEX", "STEX/ﾂ･.STEX"),
];

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
fn write_input(input: &Path, entries: &[(&[u8], Vec<u8>)]) {
    fs::create_dir_all(input).unwrap();
    let (index, payload) = hpi_bytes(entries);
    fs::write(input.join("MORI1R.HPI"), index).unwrap();
    fs::write(input.join("MORI1R.HPB"), payload).unwrap();
}

#[test]
fn cp932_names_preserve_japanese_extensions_and_multibyte_backslashes() {
    let entries: Vec<_> = NAMES.iter().map(|(raw, _)| (*raw, vec![1])).collect();
    let (index, payload) = hpi_bytes(&entries);
    let parsed = archive::hpi(&index, payload.len() as u64).unwrap();
    assert!(parsed.issues.is_empty());
    let names: Vec<_> = parsed.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        NAMES.iter().map(|(_, name)| *name).collect::<Vec<_>>()
    );
}

#[test]
fn japanese_members_extract_and_existing_ascii_corrections_keep_their_identity() {
    let t = tempfile::tempdir().unwrap();
    let input = t.path().join("in");
    let output = t.path().join("out");
    let entries: Vec<_> = NAMES
        .iter()
        .enumerate()
        .map(|(i, (raw, _))| (*raw, stex("", i as u8 + 7, 0)))
        .collect();
    write_input(&input, &entries[..1]);
    let first = extract(&input, &output).unwrap();
    let ascii = &first.assets[0];
    let mut corrections = Overrides::default();
    corrections.assets.insert(
        ascii.id.clone(),
        Override {
            category: Some(Category::Ui),
            name: Some("keep this name".into()),
            confirmed: true,
            png_orientation: None,
        },
    );
    workspace::reassign(&output, corrections).unwrap();
    write_input(&input, &entries);
    let catalog = extract(&input, &output).unwrap();
    assert_eq!(catalog.summary.unique_images, NAMES.len());
    assert!(catalog.issues.is_empty());
    let retained = catalog.assets.iter().find(|a| a.id == ascii.id).unwrap();
    assert_eq!(retained.master_file, "ui/keep_this_name.png");
    assert_eq!(retained.runtime_hashes, ascii.runtime_hashes);
    for (i, (_, name)) in NAMES.iter().enumerate() {
        let source = format!("MORI1R.HPI/{name}");
        let asset = catalog.assets.iter().find(|a| a.source == source).unwrap();
        assert_eq!(asset.png_orientation, PngOrientation::Upright);
        assert_eq!(
            asset.runtime_hashes[0],
            eotexrip::pica::runtime_hash(&entries[i].1[128..])
        );
        let master = output.join("azahar_pack_master").join(&asset.master_file);
        let deployed = output
            .join(format!("azahar_pack/{TITLE}"))
            .join(&asset.master_file);
        assert_eq!(fs::read(master).unwrap(), fs::read(deployed).unwrap());
    }
    let japanese = catalog
        .assets
        .iter()
        .find(|a| a.source.ends_with("日本.STEX"))
        .unwrap();
    assert_eq!(japanese.category.category, Category::Backgrounds);
    assert_eq!(japanese.master_file, "backgrounds/日本.png");
    let again = extract(&input, &output).unwrap();
    assert_eq!(
        serde_json::to_value(&again.assets).unwrap(),
        serde_json::to_value(&catalog.assets).unwrap()
    );
}

#[test]
fn malformed_names_are_reported_without_losing_valid_or_previous_masters() {
    let t = tempfile::tempdir().unwrap();
    let input = t.path().join("in");
    let output = t.path().join("out");
    write_input(
        &input,
        &[(NAMES[0].0, stex("", 7, 0)), (NAMES[1].0, stex("", 8, 0))],
    );
    let initial = extract(&input, &output).unwrap();
    let japanese = initial
        .assets
        .iter()
        .find(|a| a.source.ends_with("日本.STEX"))
        .unwrap();
    let master = output
        .join("azahar_pack_master")
        .join(&japanese.master_file);
    let pixels = fs::read(&master).unwrap();
    let bad: &[u8] = b"STEX/BAD_\x81"; // incomplete multibyte character
    let also_bad: &[u8] = b"STEX/BAD_\x81\x30.STEX"; // illegal trail byte
    write_input(
        &input,
        &[
            (NAMES[0].0, stex("", 7, 0)),
            (bad, stex("", 8, 0)),
            (also_bad, stex("", 9, 0)),
            (NAMES[2].0, stex("", 10, 0)),
        ],
    );
    let partial = extract(&input, &output).unwrap();
    assert_eq!(partial.summary.textures, 2);
    assert_eq!(partial.summary.unique_images, 3);
    let issues: Vec<_> = partial
        .issues
        .iter()
        .filter(|i| i.stage == "archive_member_name")
        .collect();
    assert_eq!(issues.len(), 2);
    assert_ne!(issues[0].source, issues[1].source);
    assert!(issues[0].source.ends_with("[entry 1]"));
    assert!(issues[0].message.contains("535445582f4241445f81"));
    assert!(
        issues[1]
            .message
            .contains("535445582f4241445f81302e53544558")
    );
    assert!(
        partial
            .issues
            .iter()
            .any(|i| i.stage == "preservation" && i.source == japanese.source)
    );
    assert_eq!(fs::read(&master).unwrap(), pixels);
    let before_catalog = fs::read(output.join(".eouhd/catalog.json")).unwrap();
    let before_pack = fs::read(output.join("azahar_pack_master/pack.json")).unwrap();
    write_input(&input, &[(bad, stex("", 8, 0))]);
    let error = extract(&input, &output).unwrap_err().to_string();
    assert!(error.contains("Shift-JIS"), "{error}");
    assert_eq!(
        fs::read(output.join(".eouhd/catalog.json")).unwrap(),
        before_catalog
    );
    assert_eq!(
        fs::read(output.join("azahar_pack_master/pack.json")).unwrap(),
        before_pack
    );
    assert_eq!(fs::read(master).unwrap(), pixels);
}

#[test]
fn encoded_names_still_enforce_path_and_string_bounds() {
    for raw in [
        b"STEX\\\x95\x5c\\..\\BAD.STEX".as_slice(),
        b"C:\\\x95\x5c.STEX".as_slice(),
    ] {
        let (index, payload) = hpi_bytes(&[(raw, stex("", 7, 0))]);
        assert!(archive::hpi(&index, payload.len() as u64).is_err());
    }
    let (mut index, payload) = hpi_bytes(&[(NAMES[1].0, stex("", 7, 0))]);
    index.pop(); // no NUL terminator inside the resource
    assert!(archive::hpi(&index, payload.len() as u64).is_err());
}
