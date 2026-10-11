use crate::{
    archive,
    binary::bytes,
    catalog::*,
    classify, naming, pica, resources, rom,
    workspace::{self, Workspace},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub struct Options {
    pub input: PathBuf,
    pub output: PathBuf,
    pub game: Game,
    pub title_id: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Progress {
    pub resources: usize,
    pub textures: usize,
    pub message: String,
}
struct Session<'a, F: FnMut(Progress)> {
    workspace: &'a Workspace,
    catalog: Catalog,
    images: BTreeMap<String, usize>,
    identities: BTreeMap<(String, String), usize>,
    expanded: u64,
    cancel: &'a AtomicBool,
    progress: F,
}
impl<F: FnMut(Progress)> Session<'_, F> {
    fn issue(&mut self, source: &str, stage: &str, message: impl ToString) {
        self.catalog.issues.push(Issue {
            source: source.into(),
            stage: stage.into(),
            message: message.to_string(),
        });
    }
    fn process(&mut self, source: &str, data: &[u8], depth: usize) -> Result<()> {
        ensure!(
            !self.cancel.load(Ordering::Relaxed),
            "extraction cancelled; previous outputs preserved"
        );
        ensure!(depth <= 16, "nested archive depth limit exceeded");
        self.expanded = self
            .expanded
            .checked_add(data.len() as u64)
            .context("resource byte counter overflow")?;
        ensure!(
            self.expanded <= 16 * 1024 * 1024 * 1024,
            "expanded resource byte budget exceeded"
        );
        self.catalog.summary.resources += 1;
        ensure!(
            self.catalog.summary.resources <= 200000,
            "resource-count limit exceeded"
        );
        if data.starts_with(b"ACMP") {
            let decoded = archive::acmp(data)
                .with_context(|| format!("ACMP decompression failed for {source}"))?;
            return self.process(source, &decoded, depth + 1);
        }
        if data.starts_with(b"FARC") {
            match archive::farc(data) {
                Ok(members) => {
                    for member in members {
                        self.process(
                            &format!("{source}/{}", member.name),
                            bytes(data, member.offset, member.size)?,
                            depth + 1,
                        )?;
                    }
                    return Ok(());
                }
                Err(e) => self.issue(source, "archive", e),
            }
        }
        let (textures, issues) = resources::scan(data);
        for issue in issues {
            self.issue(source, "container", issue);
        }
        if textures.is_empty() {
            if matches!(data.first(), Some(0x10 | 0x11))
                && data.len() >= 8
                && let Ok(decoded) = archive::nintendo_lz(data)
            {
                return self.process(source, &decoded, depth + 1);
            }
            if [
                ".stex", ".bam", ".bam2", ".bcmdl", ".bch", ".bcfnt", ".ctpk", ".tmx", ".ttd",
                ".tgd",
            ]
            .iter()
            .any(|ext| source.to_lowercase().ends_with(ext))
            {
                self.issue(
                    source,
                    "discovery",
                    "no supported, structurally validated image entries",
                );
            }
        }
        for texture in textures {
            match pica::decode(
                &texture.payload,
                texture.width,
                texture.height,
                texture.format,
            ) {
                Ok(pixels) => {
                    self.catalog.summary.textures += 1;
                    let mut identity = Vec::with_capacity(pixels.len() + 8);
                    identity.extend_from_slice(&texture.width.to_le_bytes());
                    identity.extend_from_slice(&texture.height.to_le_bytes());
                    identity.extend_from_slice(&pixels);
                    let image_digest = digest(&identity);
                    let hash = pica::runtime_hash(&texture.payload);
                    let ordinal = self
                        .identities
                        .entry((source.to_owned(), texture.name.clone()))
                        .or_default();
                    let id = asset_id(self.catalog.game, source, &texture.name, *ordinal);
                    *ordinal += 1;
                    let override_ = self.workspace.overrides.assets.get(&id);
                    let decision = classify::classify(
                        self.catalog.game,
                        source,
                        &texture.name,
                        texture.parser,
                        override_,
                    );
                    let (name, basis, review) =
                        naming::name(source, &texture.name, &id, decision.category, override_);
                    if let Some(&index) = self.images.get(&image_digest) {
                        let asset = &mut self.catalog.assets[index];
                        asset.origins.push(Origin {
                            id: id.clone(),
                            source: source.into(),
                            internal_name: texture.name.clone(),
                            parser: texture.parser.into(),
                        });
                        if !asset.runtime_hashes.contains(&hash) {
                            asset.runtime_hashes.push(hash);
                            asset.runtime_hashes.sort();
                        }
                        if !asset.aliases.contains(&id) && id != asset.id {
                            asset.aliases.push(id);
                            asset.aliases.sort();
                        }
                        if asset.category.category != decision.category
                            && !decision.needs_review
                            && !asset.category.needs_review
                        {
                            asset.category.category = Category::Misc;
                            asset.category.needs_review = true;
                            asset.category.grade = "shared_review".into();
                            asset.category.evidence.extend(decision.evidence);
                        }
                    } else {
                        self.workspace.save_image(
                            &image_digest,
                            texture.width,
                            texture.height,
                            &pixels,
                        )?;
                        self.images
                            .insert(image_digest.clone(), self.catalog.assets.len());
                        let origin = Origin {
                            id: id.clone(),
                            source: source.into(),
                            internal_name: texture.name.clone(),
                            parser: texture.parser.into(),
                        };
                        self.catalog.assets.push(Asset {
                            id,
                            source: source.into(),
                            internal_name: texture.name,
                            parser: texture.parser.into(),
                            width: texture.width,
                            height: texture.height,
                            format: texture.format,
                            runtime_hashes: vec![hash],
                            hash_evidence: "structural_pica_base_mip".into(),
                            image_digest,
                            png_orientation: crate::catalog::PngOrientation::Upright,
                            category: decision,
                            name,
                            name_basis: basis,
                            name_needs_review: review,
                            master_file: String::new(),
                            aliases: vec![],
                            origins: vec![origin],
                        });
                    }
                }
                Err(e) => self.issue(source, "decoder", e),
            }
        }
        (self.progress)(Progress {
            resources: self.catalog.summary.resources,
            textures: self.catalog.summary.textures,
            message: format!("Reading {source}"),
        });
        Ok(())
    }
}

pub fn extract(
    options: &Options,
    cancel: &AtomicBool,
    progress: impl FnMut(Progress),
) -> Result<Catalog> {
    ensure!(
        !options.game.research_only(),
        "Mystery Dungeon profiles need independent format validation; extraction is not claimed for these games yet"
    );
    let input = fs::canonicalize(&options.input).context("cannot open the selected input")?;
    fs::create_dir_all(&options.output)?;
    let output = fs::canonicalize(&options.output)?;
    ensure!(
        input != output && (!input.is_dir() || !output.starts_with(&input)),
        "choose an output folder outside the input tree"
    );
    let mut workspace = Workspace::open(&output, options.game)?;
    let input = rom::open(&input)?;
    let mut catalog = Catalog::new(options.game);
    catalog.title_id = options
        .title_id
        .clone()
        .or(input.title_id)
        .or_else(|| workspace.old.as_ref().and_then(|c| c.title_id.clone()));
    if let Some(title) = &catalog.title_id {
        ensure!(
            title.len() == 16 && title.bytes().all(|b| b.is_ascii_hexdigit()),
            "title ID must contain sixteen hexadecimal digits"
        );
    }
    catalog.product_code = input.product_code;
    if let Some(code) = &catalog.product_code
        && code.starts_with("CTR-P-BSK")
    {
        ensure!(
            options.game == Game::Eou,
            "this input identifies Untold; choose the matching game profile"
        );
    }
    let paths: BTreeMap<_, _> = input
        .resources
        .iter()
        .map(|r| (r.path.to_lowercase(), r))
        .collect();
    let mut consumed = BTreeSet::new();
    let mut session = Session {
        workspace: &workspace,
        catalog,
        images: BTreeMap::new(),
        identities: BTreeMap::new(),
        expanded: 0,
        cancel,
        progress,
    };
    // Pair archive indexes before reading any large payload. HPB members are
    // read directly from their disk/ROM ranges rather than expanding full HPB.
    for resource in input
        .resources
        .iter()
        .filter(|r| r.path.to_lowercase().ends_with(".hpi"))
    {
        let key = format!(
            "{}hpb",
            &resource.path.to_lowercase()[..resource.path.len() - 3]
        );
        let payload = paths
            .get(&key)
            .with_context(|| format!("{} has no matching HPB", resource.path))?;
        consumed.insert(key);
        consumed.insert(resource.path.to_lowercase());
        let members = archive::hpi(&resource.read()?, payload.size)
            .with_context(|| format!("invalid HPI index {}", resource.path))?;
        for name in members.skipped_saves {
            session.issue(
                &format!("{}/{name}", resource.path),
                "archive_save_entry",
                "Skipped save-data index entry with no bounded HPB payload; save files are not texture resources",
            );
        }
        for member in members.members {
            ensure!(
                !cancel.load(Ordering::Relaxed),
                "extraction cancelled; previous outputs preserved"
            );
            let member_resource = payload.slice(
                format!("{}/{}", resource.path, member.name),
                member.offset as u64,
                member.size as u64,
            )?;
            let raw = member_resource.read()?;
            if raw.starts_with(b"ACMP") {
                let decoded = archive::acmp(&raw)?;
                ensure!(
                    member.expanded_size == 0 || decoded.len() == member.expanded_size,
                    "HPI/ACMP expansion size mismatch"
                );
                session.process(&member_resource.path, &decoded, 1)?;
            } else {
                session.process(&member_resource.path, &raw, 0)?;
            }
        }
    }
    for resource in &input.resources {
        if consumed.contains(&resource.path.to_lowercase()) {
            continue;
        }
        let lower = resource.path.to_lowercase();
        if [".bcstm", ".bcwav", ".bcsar", ".bclim_audio", ".moflex"]
            .iter()
            .any(|ext| lower.ends_with(ext))
        {
            continue;
        }
        if resource.size > archive::MAX_RESOURCE as u64 {
            session.issue(
                &resource.path,
                "resource_limit",
                "resource exceeds 256 MiB; no bytes decoded",
            );
            continue;
        }
        session.process(&resource.path, &resource.read()?, 0)?;
    }
    ensure!(
        session.catalog.summary.textures > 0,
        "no supported textures found; previous outputs preserved"
    );
    if let Some(old) = &workspace.old {
        for asset in &old.assets {
            if !session.catalog.assets.iter().any(|new| {
                new.id == asset.id
                    || new.aliases.contains(&asset.id)
                    || new.image_digest == asset.image_digest
            }) {
                session.issue(
                    &asset.source,
                    "preservation",
                    "previous master retained because this run did not rediscover its asset",
                );
                session.catalog.assets.push(asset.clone());
            }
        }
    }
    if session.catalog.title_id.is_none() {
        session.issue("input","deployment","title ID unavailable; masters are ready, provide a title ID to create the emulator deployment");
    }
    for a in &mut session.catalog.assets {
        a.category = classify::for_asset(options.game, a, &workspace.overrides);
        let o = workspace.overrides.assets.get(&a.id).or_else(|| {
            a.aliases
                .iter()
                .find_map(|id| workspace.overrides.assets.get(id))
        });
        let (name, basis, review) =
            naming::name(&a.source, &a.internal_name, &a.id, a.category.category, o);
        a.name = name;
        a.name_basis = basis;
        a.name_needs_review = review;
    }
    naming::unique_names(&mut session.catalog.assets);
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "extraction cancelled; previous outputs preserved"
    );
    (session.progress)(Progress {
        resources: session.catalog.summary.resources,
        textures: session.catalog.summary.textures,
        message: "Publishing masters and emulator pack".into(),
    });
    let mut catalog = session.catalog;
    workspace.publish(&mut catalog)?;
    Ok(catalog)
}

#[derive(Deserialize)]
struct Bundle {
    schema: String,
    profile_id: String,
    extraction_report: HistoricalReport,
}
#[derive(Deserialize)]
struct HistoricalReport {
    title_id: String,
    product_code: String,
    textures: Vec<HistoricalTexture>,
}
#[derive(Deserialize)]
struct HistoricalTexture {
    source: String,
    #[serde(default)]
    internal_name: String,
    width: u32,
    height: u32,
    format: u32,
    parser_used: String,
    #[serde(default)]
    candidate_hash: Option<String>,
    #[serde(default)]
    azahar_eligible: bool,
}
/// Replay the extraction metadata, not the previous classifier's labels. This
/// produces a review plan; it never mutates a pack or claims pixel accuracy.
pub fn replay(path: &Path, override_path: Option<&Path>) -> Result<Catalog> {
    let bundle: Bundle = serde_json::from_reader(File::open(path)?)?;
    ensure!(
        bundle.schema == "eo-texrip-categorization-calibration-bundle-v1",
        "unsupported calibration bundle schema"
    );
    let game = match bundle.profile_id.as_str() {
        "eou1" | "eou" => Game::Eou,
        "eou2" | "eo2u" => Game::Eou2,
        "eo4" | "eoiv" => Game::Eoiv,
        "eo5" | "eov" => Game::Eov,
        "nexus" | "eon" => Game::Eon,
        _ => anyhow::bail!("unsupported calibration profile"),
    };
    let overrides: Overrides = if let Some(path) = override_path {
        serde_json::from_reader(File::open(path)?)?
    } else {
        Overrides::default()
    };
    let mut catalog = Catalog::new(game);
    catalog.title_id = Some(bundle.extraction_report.title_id);
    catalog.product_code = Some(bundle.extraction_report.product_code);
    let mut ordinals = BTreeMap::<(String, String), usize>::new();
    for t in bundle.extraction_report.textures {
        let ordinal = ordinals
            .entry((t.source.clone(), t.internal_name.clone()))
            .or_default();
        let id = asset_id(game, &t.source, &t.internal_name, *ordinal);
        *ordinal += 1;
        let o = overrides.assets.get(&id);
        let decision = classify::classify(game, &t.source, &t.internal_name, &t.parser_used, o);
        let (name, basis, review) =
            naming::name(&t.source, &t.internal_name, &id, decision.category, o);
        let hashes = if t.azahar_eligible {
            t.candidate_hash.into_iter().collect()
        } else {
            vec![]
        };
        let origin = Origin {
            id: id.clone(),
            source: t.source.clone(),
            internal_name: t.internal_name.clone(),
            parser: t.parser_used.clone(),
        };
        catalog.assets.push(Asset {
            id,
            source: t.source,
            internal_name: t.internal_name,
            parser: t.parser_used,
            width: t.width,
            height: t.height,
            format: t.format,
            runtime_hashes: hashes,
            hash_evidence: "historical_metadata_only".into(),
            image_digest: String::new(),
            png_orientation: crate::catalog::PngOrientation::Upright,
            category: decision,
            name,
            name_basis: basis,
            name_needs_review: review,
            master_file: String::new(),
            aliases: vec![],
            origins: vec![origin],
        });
    }
    naming::unique_names(&mut catalog.assets);
    for a in &mut catalog.assets {
        a.master_file = format!(
            "{}{}/{}.png",
            if a.runtime_hashes.is_empty() {
                "non-azahar/"
            } else {
                ""
            },
            a.category.category.folder(),
            a.name
        );
    }
    catalog.summary.textures = catalog.assets.len();
    catalog.recount();
    Ok(catalog)
}

pub fn export(root: &Path, cancel: &AtomicBool) -> Result<Catalog> {
    ensure!(!cancel.load(Ordering::Relaxed), "pack rebuild cancelled");
    let mut catalog = workspace::load(root)?;
    let mut workspace = Workspace::open(root, catalog.game)?;
    workspace.publish(&mut catalog)?;
    Ok(catalog)
}
