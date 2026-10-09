use crate::{
    binary::logical_path,
    catalog::{Catalog, Overrides},
    naming,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize)]
struct Swap {
    target: String,
    stage: String,
    backup: String,
    existed: bool,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    job: String,
    swaps: Vec<Swap>,
    committed: bool,
}
pub struct Workspace {
    pub root: PathBuf,
    pub job: PathBuf,
    pub old: Option<Catalog>,
    pub overrides: Overrides,
    _lock: File,
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(path.parent().context("metadata path has no parent")?)?;
    let temporary = path.with_extension("json.new");
    let mut file = File::create(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    // Metadata outside a swap (journal/overrides) uses a removable temporary;
    // Windows rename does not replace an existing file. Journal replacement
    // goes through a persistent .previous file, recovered on next open.
    let previous = path.with_extension("json.previous");
    if previous.exists() {
        fs::remove_file(&previous)?;
    }
    if path.exists() {
        fs::rename(path, &previous)?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if previous.exists() {
            let _ = fs::rename(previous, path);
        }
        return Err(error.into());
    }
    if previous.exists() {
        fs::remove_file(previous)?;
    }
    Ok(())
}
fn require_plain(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path).is_ok() {
        ensure!(
            !fs::symlink_metadata(path)?.file_type().is_symlink(),
            "workspace contains a symbolic link: {}",
            path.display()
        );
    }
    Ok(())
}
fn remove_owned(path: &Path) -> Result<()> {
    require_plain(path)?;
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    if !from.exists() {
        return Ok(());
    }
    require_plain(from)?;
    for entry in walkdir::WalkDir::new(from).follow_links(false) {
        let entry = entry?;
        ensure!(
            !entry.file_type().is_symlink(),
            "workspace contains symbolic links"
        );
        let target = to.join(entry.path().strip_prefix(from)?);
        if entry.file_type().is_dir() {
            fs::create_dir_all(target)?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
fn safe_relative(value: &str) -> Result<String> {
    let clean = logical_path(value)?;
    ensure!(clean == value, "noncanonical workspace path");
    Ok(clean)
}
fn recover(root: &Path) -> Result<()> {
    require_plain(&root.join(".eouhd/jobs"))?;
    let path = root.join(".eouhd/journal.json");
    require_plain(&path)?;
    let previous = path.with_extension("json.previous");
    require_plain(&previous)?;
    if !path.exists() && previous.exists() {
        fs::rename(&previous, &path)?;
    }
    if !path.exists() {
        return Ok(());
    }
    let journal: Journal = serde_json::from_reader(File::open(&path)?)?;
    let job = safe_relative(&journal.job)?;
    ensure!(
        job.starts_with(".eouhd/jobs/") && job.split('/').count() == 3,
        "invalid recovery job"
    );
    require_plain(&root.join(&job))?;
    require_plain(&root.join(&job).join("backup"))?;
    if !journal.committed {
        for swap in journal.swaps.iter().rev() {
            let target = safe_relative(&swap.target)?;
            ensure!(
                target == "azahar_pack_master"
                    || target == "azahar_pack"
                    || target == ".eouhd/catalog.json"
                    || target == ".eouhd/overrides.json"
                    || target.starts_with(".eouhd/") && target.ends_with("-extraction-report.json"),
                "invalid recovery target"
            );
            let stage = safe_relative(&swap.stage)?;
            let backup = safe_relative(&swap.backup)?;
            ensure!(
                stage.starts_with(&format!("{job}/")) && backup.starts_with(&format!("{job}/")),
                "recovery escapes job"
            );
            let destination = root.join(target);
            let backup = root.join(backup);
            let stage = root.join(stage);
            require_plain(&destination)?;
            require_plain(&backup)?;
            require_plain(&stage)?;
            if backup.exists() {
                remove_owned(&destination)?;
                fs::rename(backup, destination)?;
            } else if !swap.existed && !stage.exists() {
                remove_owned(&destination)?;
            }
        }
    }
    fs::remove_file(&path)?;
    if previous.exists() {
        fs::remove_file(previous)?;
    }
    remove_owned(&root.join(job))?;
    Ok(())
}

impl Workspace {
    pub fn open(root: &Path, game: crate::catalog::Game) -> Result<Self> {
        require_plain(root)?;
        fs::create_dir_all(root)?;
        let root = fs::canonicalize(root)?;
        require_plain(&root.join(".eouhd"))?;
        fs::create_dir_all(root.join(".eouhd"))?;
        require_plain(&root.join(".eouhd/workspace.lock"))?;
        require_plain(&root.join(".eouhd/jobs"))?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".eouhd/workspace.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).context("another job is using this workspace")?;
        recover(&root)?;
        for managed in [
            "azahar_pack_master",
            "azahar_pack",
            ".eouhd/catalog.json",
            ".eouhd/overrides.json",
        ] {
            require_plain(&root.join(managed))?;
        }
        let catalog = root.join(".eouhd/catalog.json");
        let old: Option<Catalog> = if catalog.exists() {
            Some(
                serde_json::from_reader(File::open(catalog)?)
                    .context("catalog is damaged; existing masters have been preserved")?,
            )
        } else {
            None
        };
        if let Some(old) = &old {
            ensure!(
                old.schema == "eotexrip-catalog-v1" && old.game == game,
                "workspace belongs to another game or catalog version"
            );
            let mut ids = std::collections::BTreeSet::new();
            for a in &old.assets {
                safe_relative(&a.master_file)?;
                ensure!(ids.insert(&a.id), "catalog has duplicate asset identities");
            }
        } else {
            ensure!(
                !root.join("azahar_pack_master").exists() && !root.join("azahar_pack").exists(),
                "existing image outputs have no valid catalog; select a new workspace"
            );
        }
        let override_path = root.join(".eouhd/overrides.json");
        let overrides = if override_path.exists() {
            serde_json::from_reader(File::open(override_path)?)
                .context("saved overrides are damaged; outputs have been preserved")?
        } else {
            Overrides::default()
        };
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let job = root.join(format!(".eouhd/jobs/{}_{stamp}", std::process::id()));
        fs::create_dir_all(job.join("images"))?;
        Ok(Self {
            root,
            job,
            old,
            overrides,
            _lock: lock,
        })
    }
    pub fn image_path(&self, digest: &str) -> PathBuf {
        self.job.join("images").join(format!("{digest}.png"))
    }
    pub fn save_image(&self, digest: &str, width: u32, height: u32, pixels: &[u8]) -> Result<()> {
        let path = self.image_path(digest);
        if path.exists() {
            return Ok(());
        }
        let file = BufWriter::new(File::create(path)?);
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(pixels)?;
        Ok(())
    }
    pub fn publish(&mut self, catalog: &mut Catalog) -> Result<()> {
        if let Some(old) = &self.old {
            ensure!(
                old.title_id.is_none()
                    || catalog.title_id.is_none()
                    || old.title_id == catalog.title_id,
                "input title/region differs from the existing workspace"
            );
        }
        let master = self.job.join("masters");
        let deploy = self.job.join("deployment");
        copy_tree(&self.root.join("azahar_pack_master"), &master)?;
        fs::create_dir_all(&deploy)?;
        let pack_path = master.join("pack.json");
        let old_pack: serde_json::Value = if pack_path.exists() {
            serde_json::from_reader(File::open(&pack_path)?)
                .context("master pack.json is damaged; masters preserved")?
        } else {
            serde_json::json!({})
        };
        let mut inventory = BTreeMap::<String, Vec<String>>::new();
        for entry in walkdir::WalkDir::new(&master).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("png"))
            {
                let relative = entry
                    .path()
                    .strip_prefix(&master)?
                    .to_string_lossy()
                    .replace('\\', "/");
                inventory
                    .entry(entry.file_name().to_string_lossy().to_lowercase())
                    .or_default()
                    .push(relative);
            }
        }
        let mut effective_overrides = self.overrides.clone();
        for asset in &mut catalog.assets {
            let explicit = self
                .overrides
                .assets
                .get(&asset.id)
                .is_some_and(|o| o.confirmed && (o.name.is_some() || o.category.is_some()));
            let old = self.old.as_ref().and_then(|c| {
                c.assets.iter().find(|a| {
                    a.id == asset.id
                        || a.aliases.contains(&asset.id)
                        || asset.aliases.contains(&a.id)
                        || a.image_digest == asset.image_digest
                })
            });
            let mut existing = old
                .map(|a| a.master_file.clone())
                .filter(|p| master.join(p).is_file());
            if let Some(old) = old {
                for hash in &old.runtime_hashes {
                    if let Some(value) = old_pack
                        .get("textures")
                        .and_then(|m| m.get(hash))
                        .and_then(|v| v.as_str())
                    {
                        let clean = logical_path(value)?;
                        if let Some(paths) =
                            inventory.get(&clean.rsplit('/').next().unwrap_or("").to_lowercase())
                        {
                            ensure!(
                                paths.len() == 1,
                                "manual mapping {value} has ambiguous filenames"
                            );
                            if !explicit
                                && (paths[0] != old.master_file
                                    || old.name_basis == "manual_pack_mapping")
                            {
                                // A changed live mapping is a user's actual rename/category choice.
                                existing = Some(paths[0].clone());
                                asset.name = paths[0]
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or("")
                                    .trim_end_matches(".png")
                                    .into();
                                asset.name_basis = "manual_pack_mapping".into();
                                asset.name_needs_review = false;
                                if let Some(category) = crate::catalog::Category::ALL
                                    .into_iter()
                                    .find(|c| paths[0].starts_with(&format!("{}/", c.folder())))
                                {
                                    asset.category.category = category;
                                    asset.category.grade = "confirmed".into();
                                    asset.category.needs_review = false;
                                }
                                let choice = crate::catalog::Override {
                                    category: Some(asset.category.category),
                                    name: Some(asset.name.clone()),
                                    confirmed: true,
                                };
                                effective_overrides
                                    .assets
                                    .insert(asset.id.clone(), choice.clone());
                                for alias in &asset.aliases {
                                    effective_overrides
                                        .assets
                                        .insert(alias.clone(), choice.clone());
                                }
                            }
                        }
                    }
                }
                ensure!(
                    existing.is_some(),
                    "previous master {} is missing; update its live pack mapping before rebuilding",
                    old.master_file
                );
            }
            let eligible = !asset.runtime_hashes.is_empty();
            let prefix = if eligible { "" } else { "non-azahar/" };
            let mut relative = format!(
                "{prefix}{}/{}.png",
                asset.category.category.folder(),
                asset.name
            );
            if asset.name_basis == "manual_pack_mapping" {
                relative = existing.clone().unwrap();
            }
            safe_relative(&relative)?;
            if master.join(&relative).exists() && existing.as_ref() != Some(&relative) {
                asset.name = format!("{}_{}", asset.name, asset.id);
                relative = format!(
                    "{prefix}{}/{}.png",
                    asset.category.category.folder(),
                    asset.name
                );
                ensure!(
                    !master.join(&relative).exists(),
                    "output filename collides with an existing master"
                );
            }
            fs::create_dir_all(master.join(&relative).parent().unwrap())?;
            if let Some(existing) = existing {
                if existing != relative {
                    fs::rename(master.join(existing), master.join(&relative))?;
                }
            } else {
                fs::copy(self.image_path(&asset.image_digest), master.join(&relative))?;
            }
            asset.master_file = relative;
        }
        let mut mapping = BTreeMap::<String, String>::new();
        let mut names = BTreeMap::<String, String>::new();
        for asset in &catalog.assets {
            let filename = asset
                .master_file
                .rsplit('/')
                .next()
                .context("empty master path")?
                .to_owned();
            if let Some(prior) = names.insert(filename.to_lowercase(), asset.master_file.clone()) {
                ensure!(
                    prior == asset.master_file,
                    "master basenames are ambiguous across categories"
                );
            }
            for hash in &asset.runtime_hashes {
                ensure!(
                    hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                    "invalid runtime hash"
                );
                if let Some(prior) = mapping.insert(hash.clone(), filename.clone()) {
                    ensure!(
                        prior == filename,
                        "one runtime hash maps to conflicting images"
                    );
                }
            }
        }
        let mut pack = old_pack;
        ensure!(pack.is_object(), "pack.json must be an object");
        pack["name"] = serde_json::json!(format!("{} textures", catalog.game.label()));
        pack["version"] = serde_json::json!(crate::VERSION);
        pack["options"] =
            serde_json::json!({"skip_mipmap":false,"flip_png_files":true,"use_new_hash":true});
        // Preserve valid additional user mappings, but never deploy PNG-only
        // non-azahar content or a mapping to a missing/ambiguous master.
        if let Some(user) = pack.get("textures").and_then(|v| v.as_object()) {
            for (hash, value) in user {
                if mapping.contains_key(hash)
                    || hash.len() != 16
                    || !hash.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    continue;
                }
                if let Some(name) = value.as_str()
                    && let Some(paths) = inventory.get(&name.to_lowercase())
                    && paths.len() == 1
                    && !paths[0].starts_with("non-azahar/")
                    && master.join(&paths[0]).is_file()
                {
                    mapping.insert(hash.clone(), name.into());
                }
            }
        }
        pack["textures"] = serde_json::to_value(&mapping)?;
        write_json(&pack_path, &pack)?;
        if let Some(title) = &catalog.title_id {
            ensure!(
                title.len() == 16 && title.bytes().all(|x| x.is_ascii_hexdigit()),
                "title ID must contain sixteen hexadecimal digits"
            );
            let target = deploy.join(title);
            fs::create_dir_all(&target)?;
            let mut files = BTreeMap::new();
            for entry in walkdir::WalkDir::new(&master) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let relative = entry
                        .path()
                        .strip_prefix(&master)?
                        .to_string_lossy()
                        .replace('\\', "/");
                    if !relative.starts_with("non-azahar/") {
                        files
                            .entry(entry.file_name().to_string_lossy().to_lowercase())
                            .or_insert_with(Vec::new)
                            .push(relative);
                    }
                }
            }
            for name in mapping.values() {
                let paths = files
                    .get(&name.to_lowercase())
                    .context("pack mapping references a missing master")?;
                ensure!(paths.len() == 1, "pack mapping filename is ambiguous");
                let relative = &paths[0];
                let destination = target.join(relative);
                fs::create_dir_all(destination.parent().unwrap())?;
                fs::copy(master.join(relative), destination)?;
            }
            write_json(&target.join("pack.json"), &pack)?;
        }
        catalog.recount();
        write_json(&self.job.join("overrides.json"), &effective_overrides)?;
        write_json(&self.job.join("catalog.json"), catalog)?;
        write_json(&self.job.join("report.json"), catalog)?;
        let job = self
            .job
            .strip_prefix(&self.root)?
            .to_string_lossy()
            .replace('\\', "/");
        let targets = [
            ("azahar_pack_master".to_string(), "masters"),
            ("azahar_pack".into(), "deployment"),
            (".eouhd/catalog.json".into(), "catalog.json"),
            (".eouhd/overrides.json".into(), "overrides.json"),
            (
                format!(".eouhd/{}-extraction-report.json", catalog.game.prefix()),
                "report.json",
            ),
        ];
        let swaps = targets
            .into_iter()
            .enumerate()
            .map(|(i, (target, stage))| Swap {
                existed: self.root.join(&target).exists(),
                target,
                stage: format!("{job}/{stage}"),
                backup: format!("{job}/backup/{i}"),
            })
            .collect();
        let mut journal = Journal {
            job,
            swaps,
            committed: false,
        };
        let journal_path = self.root.join(".eouhd/journal.json");
        write_json(&journal_path, &journal)?;
        fs::create_dir_all(self.job.join("backup"))?;
        let result = (|| -> Result<()> {
            for swap in &journal.swaps {
                let destination = self.root.join(&swap.target);
                if swap.existed {
                    fs::rename(&destination, self.root.join(&swap.backup))?;
                }
                fs::rename(self.root.join(&swap.stage), destination)?;
            }
            journal.committed = true;
            write_json(&journal_path, &journal)?;
            Ok(())
        })();
        if let Err(error) = result {
            recover(&self.root).context("failed to roll back interrupted publication")?;
            return Err(error);
        }
        recover(&self.root)?;
        self.overrides = effective_overrides;
        Ok(())
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        if !self.root.join(".eouhd/journal.json").exists() {
            let _ = remove_owned(&self.job);
        }
    }
}

pub fn load(root: &Path) -> Result<Catalog> {
    let catalog = serde_json::from_reader(File::open(root.join(".eouhd/catalog.json"))?)?;
    Ok(catalog)
}
pub fn save_plan(path: &Path, catalog: &Catalog) -> Result<()> {
    write_json(path, catalog)
}
pub fn save_choices(path: &Path, choices: &Overrides) -> Result<()> {
    write_json(path, choices)
}

pub fn reassign(root: &Path, overrides: Overrides) -> Result<Catalog> {
    let current = load(root)?;
    let mut workspace = Workspace::open(root, current.game)?;
    let mut catalog = current;
    for id in overrides.assets.keys() {
        ensure!(
            catalog
                .assets
                .iter()
                .any(|a| &a.id == id || a.aliases.contains(id)),
            "correction identity {id} is not present in this workspace"
        );
    }
    for (id, choice) in overrides.assets {
        workspace.overrides.assets.insert(id, choice);
    }
    for asset in &mut catalog.assets {
        let o = workspace.overrides.assets.get(&asset.id);
        asset.category = crate::classify::for_asset(catalog.game, asset, &workspace.overrides);
        let (name, basis, review) = naming::name(
            &asset.source,
            &asset.internal_name,
            &asset.id,
            asset.category.category,
            o,
        );
        asset.name = name;
        asset.name_basis = basis;
        asset.name_needs_review = review;
    }
    naming::unique_names(&mut catalog.assets);
    workspace.publish(&mut catalog)?;
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_directory_swap_recovers_previous_publication() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let job = ".eouhd/jobs/recovery";
        fs::create_dir_all(root.join(format!("{job}/backup"))).unwrap();
        fs::create_dir_all(root.join("azahar_pack_master")).unwrap();
        fs::write(root.join("azahar_pack_master/new.png"), b"new").unwrap();
        fs::create_dir_all(root.join(format!("{job}/backup/0"))).unwrap();
        fs::write(root.join(format!("{job}/backup/0/edited.png")), b"edited").unwrap();
        let journal = Journal {
            job: job.into(),
            committed: false,
            swaps: vec![Swap {
                target: "azahar_pack_master".into(),
                stage: format!("{job}/masters"),
                backup: format!("{job}/backup/0"),
                existed: true,
            }],
        };
        write_json(&root.join(".eouhd/journal.json"), &journal).unwrap();
        recover(root).unwrap();
        assert_eq!(
            fs::read(root.join("azahar_pack_master/edited.png")).unwrap(),
            b"edited"
        );
        assert!(!root.join("azahar_pack_master/new.png").exists());
        assert!(!root.join(job).exists());
    }
    #[test]
    fn malicious_recovery_path_does_not_touch_external_files() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let job = ".eouhd/jobs/recovery";
        fs::create_dir_all(root.join(job)).unwrap();
        let journal = Journal {
            job: job.into(),
            committed: false,
            swaps: vec![Swap {
                target: "../outside".into(),
                stage: format!("{job}/masters"),
                backup: format!("{job}/backup/0"),
                existed: true,
            }],
        };
        write_json(&root.join(".eouhd/journal.json"), &journal).unwrap();
        assert!(recover(root).is_err());
    }
}
