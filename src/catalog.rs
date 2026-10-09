use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Game {
    #[default]
    Eou,
    Eou2,
    Eoiv,
    Eov,
    Eon,
    Emd,
    Emd2,
}
impl Game {
    pub const ALL: [Self; 7] = [
        Self::Eou,
        Self::Eou2,
        Self::Eoiv,
        Self::Eov,
        Self::Eon,
        Self::Emd,
        Self::Emd2,
    ];
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Eou => "EOU",
            Self::Eou2 => "EOU2",
            Self::Eoiv => "EOIV",
            Self::Eov => "EOV",
            Self::Eon => "EON",
            Self::Emd => "EMD",
            Self::Emd2 => "EMD2",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Eou => "Etrian Odyssey Untold",
            Self::Eou2 => "Etrian Odyssey 2 Untold",
            Self::Eoiv => "Etrian Odyssey IV",
            Self::Eov => "Etrian Odyssey V",
            Self::Eon => "Etrian Odyssey Nexus",
            Self::Emd => "Etrian Mystery Dungeon",
            Self::Emd2 => "Etrian Mystery Dungeon 2",
        }
    }
    pub fn research_only(self) -> bool {
        matches!(self, Self::Emd | Self::Emd2)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Characters,
    Monsters,
    Ui,
    Icons,
    Maps,
    Dungeon,
    Backgrounds,
    Effects,
    Fonts,
    #[default]
    Misc,
}
impl Category {
    pub const ALL: [Self; 10] = [
        Self::Characters,
        Self::Monsters,
        Self::Ui,
        Self::Icons,
        Self::Maps,
        Self::Dungeon,
        Self::Backgrounds,
        Self::Effects,
        Self::Fonts,
        Self::Misc,
    ];
    pub fn folder(self) -> &'static str {
        match self {
            Self::Characters => "characters",
            Self::Monsters => "monsters",
            Self::Ui => "ui",
            Self::Icons => "icons",
            Self::Maps => "maps",
            Self::Dungeon => "dungeon",
            Self::Backgrounds => "backgrounds",
            Self::Effects => "effects",
            Self::Fonts => "fonts",
            Self::Misc => "misc",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub category: Category,
    pub strength: u16,
    pub rule: String,
    pub detail: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Decision {
    pub category: Category,
    /// Evidence grade, not a statistically calibrated probability.
    pub grade: String,
    pub needs_review: bool,
    pub evidence: Vec<Evidence>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Override {
    pub category: Option<Category>,
    pub name: Option<String>,
    #[serde(default)]
    pub confirmed: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub assets: BTreeMap<String, Override>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Origin {
    pub id: String,
    pub source: String,
    pub internal_name: String,
    pub parser: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub source: String,
    pub internal_name: String,
    pub parser: String,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub runtime_hashes: Vec<String>,
    pub hash_evidence: String,
    pub image_digest: String,
    pub category: Decision,
    pub name: String,
    pub name_basis: String,
    pub name_needs_review: bool,
    pub master_file: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub origins: Vec<Origin>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Issue {
    pub source: String,
    pub stage: String,
    pub message: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub resources: usize,
    pub textures: usize,
    pub unique_images: usize,
    pub mapped_images: usize,
    pub png_only_images: usize,
    pub category_review: usize,
    pub name_review: usize,
    pub by_category: BTreeMap<String, usize>,
    pub issues: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: String,
    pub version: String,
    pub game: Game,
    pub title_id: Option<String>,
    pub product_code: Option<String>,
    pub assets: Vec<Asset>,
    pub issues: Vec<Issue>,
    pub summary: Summary,
}
impl Catalog {
    pub fn new(game: Game) -> Self {
        Self {
            schema: "eotexrip-catalog-v1".into(),
            version: crate::VERSION.into(),
            game,
            title_id: None,
            product_code: None,
            assets: vec![],
            issues: vec![],
            summary: Summary::default(),
        }
    }
    pub fn recount(&mut self) {
        let resources = self.summary.resources;
        let textures = self.summary.textures;
        self.summary = Summary {
            resources,
            textures,
            unique_images: self.assets.len(),
            issues: self.issues.len(),
            ..Summary::default()
        };
        for asset in &self.assets {
            *self
                .summary
                .by_category
                .entry(asset.category.category.folder().into())
                .or_default() += 1;
            self.summary.category_review += usize::from(asset.category.needs_review);
            self.summary.name_review += usize::from(asset.name_needs_review);
            if asset.runtime_hashes.is_empty() {
                self.summary.png_only_images += 1;
            } else {
                self.summary.mapped_images += 1;
            }
        }
    }
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn asset_id(game: Game, source: &str, name: &str, ordinal: usize) -> String {
    digest(
        format!(
            "{}\0{}\0{}\0{ordinal}",
            game.prefix(),
            source.replace("::", "/").replace('\\', "/").to_lowercase(),
            name
        )
        .as_bytes(),
    )[..24]
        .to_owned()
}
