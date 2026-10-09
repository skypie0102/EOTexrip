use crate::catalog::{Asset, Category, Override};
use std::collections::BTreeMap;

pub fn logical_name(value: &str) -> String {
    let leaf = value.rsplit(['/', '\\']).next().unwrap_or(value);
    let stem = leaf.rsplit_once('.').map(|(s, _)| s).unwrap_or(leaf);
    // Strip only a complete known export suffix, not a meaningful double '_'.
    if let Some((name, tail)) = stem.rsplit_once("__") {
        let parts: Vec<_> = tail.split('_').collect();
        if parts.len() == 3
            && parts[0]
                .split_once('x')
                .is_some_and(|(a, b)| a.parse::<u32>().is_ok() && b.parse::<u32>().is_ok())
            && parts[1]
                .strip_prefix('f')
                .is_some_and(|s| s.parse::<u32>().is_ok())
            && parts[2].len() == 16
            && parts[2].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return name.into();
        }
    }
    stem.into()
}

pub fn safe_stem(value: &str) -> String {
    let mut result = String::new();
    for c in value.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
        } else if c.is_alphanumeric() {
            result.push(c);
        } else if !result.ends_with('_') {
            result.push('_');
        }
        if result.len() >= 100 {
            break;
        }
    }
    let mut result = result.trim_matches('_').to_owned();
    if result.is_empty() {
        result = "texture".into();
    }
    let reserved = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if reserved.contains(&result.as_str()) {
        result.push_str("_texture");
    }
    result
}

pub fn name(
    source: &str,
    internal: &str,
    id: &str,
    category: Category,
    override_: Option<&Override>,
) -> (String, String, bool) {
    if let Some(o) =
        override_.filter(|o| o.confirmed && o.name.as_ref().is_some_and(|n| !n.trim().is_empty()))
    {
        return (
            safe_stem(o.name.as_ref().unwrap()),
            "user_confirmed".into(),
            false,
        );
    }
    let source_stem = logical_name(source);
    let raw = if internal.is_empty() {
        source_stem.as_str()
    } else {
        internal
    };
    let raw = logical_name(raw);
    let mut label = safe_stem(&raw);
    let generic = [
        "texture",
        "stex_texture",
        "image",
        "default",
        "unnamed",
        "unknown",
    ];
    let coded = label.bytes().all(|c| c.is_ascii_digit() || c == b'_')
        || label.starts_with("cgfx_tex_")
        || label.starts_with("bch_tex_");
    let generic_name = generic.contains(&label.as_str()) || coded;
    // A filename such as NPC33 or EN180A identifies a source, but does not
    // establish the person's or enemy's readable identity.
    let identity_code = label.split('_').any(|token| {
        ["npc", "pc", "cha", "en"].iter().any(|p| {
            token
                .strip_prefix(p)
                .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit()))
        })
    });
    let review = generic_name || identity_code;
    if label.starts_with("ig_") {
        label = label[3..].to_owned();
    }
    let prefixes = [
        ("key_", "keyboard_"),
        ("fac_", "facility_"),
        ("bat_", "battle_"),
        ("res_", "result_"),
        ("cam_", "camp_"),
        ("opt_", "options_"),
        ("dmap_", "dungeon_map_"),
        ("eve_", "event_"),
        ("ev_", "event_"),
        ("co_", "common_"),
        ("sta_", "status_"),
        ("cus_", "customize_"),
        ("boo_", "book_"),
    ];
    for (from, to) in prefixes {
        if let Some(tail) = label.strip_prefix(from) {
            label = format!("{to}{tail}");
            break;
        }
    }
    if let Some(tail) = label.strip_prefix("en_") {
        label = format!("enemy_{tail}");
    }
    // Effects frequently repeat generic particle names. Include their actual
    // source-owner stem, without inventing the English name of a spell.
    if category == Category::Effects {
        let owner = source
            .replace("::", "/")
            .replace('\\', "/")
            .split('/')
            .find(|s| s.to_lowercase().ends_with(".epl"))
            .map(logical_name);
        if let Some(owner) = owner {
            let owner = safe_stem(&owner);
            if !label.starts_with(&owner) {
                label = format!("{owner}_{label}");
            }
        }
    }
    if category == Category::Fonts && label.starts_with("font_sheet_") {
        label = format!("{}_{}", safe_stem(&source_stem), label);
    }
    if category == Category::Characters {
        let parts: Vec<_> = internal.split(['/', '\\']).collect();
        if let Some(owner) = parts
            .iter()
            .take(parts.len().saturating_sub(1))
            .find_map(|p| {
                p.split_once('_')
                    .filter(|(n, s)| {
                        !n.is_empty()
                            && n.bytes().all(|b| b.is_ascii_digit())
                            && s.chars().all(char::is_alphanumeric)
                    })
                    .map(|(_, s)| s)
            })
        {
            label = format!("{}_{}", safe_stem(owner), label);
        }
    }
    if generic_name {
        label = format!("{}_{}", category.folder(), &id[..id.len().min(10)]);
    }
    (
        safe_stem(&label),
        if internal.is_empty() {
            "source_name"
        } else {
            "internal_name"
        }
        .into(),
        review,
    )
}

/// Globally unique basenames; every collision gets a stable suffix so input
/// order cannot change which image wins a friendly name.
pub fn unique_names(assets: &mut [Asset]) {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (i, a) in assets.iter().enumerate() {
        groups.entry(a.name.to_lowercase()).or_default().push(i);
    }
    for indices in groups.values().filter(|g| g.len() > 1) {
        for &i in indices {
            let a = &mut assets[i];
            a.name = format!("{}_{}", a.name, &a.id[..a.id.len().min(12)]);
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for a in assets {
        if !seen.insert(a.name.to_lowercase()) {
            a.name = format!("{}_{}", a.name, a.id);
            seen.insert(a.name.to_lowercase());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_names_and_keep_real_identity() {
        assert_eq!(
            logical_name("bf05_floor01__64x64_f12_421A4257CD1E07B5.png"),
            "bf05_floor01"
        );
        assert_eq!(
            logical_name("name__meaningful_part.png"),
            "name__meaningful_part"
        );
        assert_eq!(
            name(
                "STEX/KEYBOARD/IG_KEY_KATAKANA045.STEX",
                "",
                "1234567890",
                Category::Fonts,
                None
            )
            .0,
            "keyboard_katakana045"
        );
        assert_eq!(
            name(
                "MONSTER/EN123.BAM",
                "en_goldhorn_t01",
                "1234567890",
                Category::Monsters,
                None
            )
            .0,
            "enemy_goldhorn_t01"
        );
        assert_eq!(safe_stem("../CON"), "con_texture");
    }
}
