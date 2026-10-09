use crate::catalog::{Category as C, Decision, Evidence, Game, Override};
use std::collections::BTreeMap;

fn numbered(name: &str, prefix: &str) -> bool {
    let tail = name.strip_prefix(prefix).unwrap_or("");
    let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
    digits >= 2
        && tail
            .as_bytes()
            .get(digits)
            .is_some_and(|c| matches!(c, b'_' | b'-'))
}
fn token_family(token: &str, prefix: &str) -> bool {
    token
        .strip_prefix(prefix)
        .is_some_and(|s| s.is_empty() || s.bytes().all(|b| b.is_ascii_digit()))
}

/// Classify original provenance, never the previous output folder or its label.
pub fn classify(
    game: Game,
    source: &str,
    internal_name: &str,
    parser: &str,
    override_: Option<&Override>,
) -> Decision {
    if let Some(o) = override_.filter(|o| o.confirmed && o.category.is_some()) {
        return Decision {
            category: o.category.unwrap(),
            grade: "confirmed".into(),
            needs_review: false,
            evidence: vec![Evidence {
                category: o.category.unwrap(),
                strength: 1000,
                rule: "user_confirmed".into(),
                detail: "Saved asset override".into(),
            }],
        };
    }
    let path = source.replace("::", "/").replace('\\', "/").to_lowercase();
    let internal_path = internal_name.replace('\\', "/").to_lowercase();
    let parts: Vec<&str> = path.split('/').collect();
    let mut parent: Vec<&str> = parts[..parts.len().saturating_sub(1)]
        .iter()
        .map(|p| p.trim_end_matches('_'))
        .collect();
    let internal_parts: Vec<_> = internal_path.split('/').collect();
    parent.extend(
        internal_parts[..internal_parts.len().saturating_sub(1)]
            .iter()
            .map(|p| p.trim_end_matches('_')),
    );
    let leaf = crate::naming::logical_name(parts.last().copied().unwrap_or(""));
    let stex = parser.contains("stex") || path.ends_with(".stex");
    let has = |names: &[&str]| parent.iter().any(|p| names.contains(p));
    let logical = crate::naming::logical_name(if internal_name.is_empty() {
        parts.last().copied().unwrap_or("")
    } else {
        internal_name
    })
    .to_lowercase();
    let words: Vec<&str> = logical.split(['_', '-', ' ']).collect();
    let model = matches!(parser, "cgfx" | "bch" | "cgfx_struct" | "bch_struct")
        || path.ends_with(".bam")
        || path.ends_with(".bam2")
        || path.ends_with(".bcmdl")
        || path.ends_with(".bch");
    let effect = has(&[
        "effect",
        "effects",
        "effect_editor",
        "vfx",
        "particle",
        "particles",
    ]) || parts.iter().any(|p| p.ends_with(".epl"));
    let icon = has(&["icon", "icons", "mapicon"])
        || logical.starts_with("ig_") && words.iter().any(|w| token_family(w, "icon"));
    let mut evidence = Vec::new();
    let mut add = |category, strength, rule: &str, detail: &str| {
        evidence.push(Evidence {
            category,
            strength,
            rule: rule.into(),
            detail: detail.into(),
        })
    };
    if parser == "bcfnt" || path.ends_with(".bcfnt") || path.ends_with(".bffnt") {
        add(C::Fonts, 130, "font_container", "Font sheet descriptor");
    }
    if has(&["font", "fonts"]) {
        add(C::Fonts, 120, "font_directory", "Exact font path component");
    }
    if stex
        && logical.starts_with("ig_key_")
        && words.iter().any(|w| {
            ["hiragana", "katakana", "oomoji", "eiji"]
                .iter()
                .any(|p| token_family(w, p))
        })
    {
        add(
            C::Fonts,
            125,
            "keyboard_glyph_name",
            "Anchored keyboard glyph identity",
        );
    }
    if has(&["keyboard"]) {
        if has(&["hiragana", "katakana", "oomoji", "eiji", "sign"])
            || words.iter().any(|w| {
                w.starts_with("hiragana")
                    || w.starts_with("katakana")
                    || w.starts_with("oomoji")
                    || w.starts_with("eiji")
            })
        {
            add(C::Fonts, 125, "keyboard_glyph", "Keyboard glyph family");
        } else {
            add(C::Ui, 110, "keyboard_interface", "Keyboard interface parts");
        }
    }
    if effect {
        add(
            C::Effects,
            125,
            "effect_directory",
            "Exact effect resource family",
        );
    }
    if has(&[
        "monster", "monsters", "enemy", "enemies", "foe", "foes", "boss", "bosses",
    ]) && !effect
    {
        add(
            C::Monsters,
            120,
            "enemy_directory",
            "Exact enemy/FOE resource family",
        );
    }
    if has(&[
        "chara",
        "charadata",
        "chara_cam",
        "character",
        "characters",
        "portrait",
        "portraits",
        "npc",
        "player",
    ]) && !effect
    {
        add(
            C::Characters,
            120,
            "character_directory",
            "Character/portrait source family",
        );
    }
    if has(&["battlefield"]) {
        add(
            C::Dungeon,
            125,
            "battlefield_directory",
            "Battlefield geometry source family",
        );
    }
    if has(&["dungeon"]) && !stex && !effect {
        add(
            C::Dungeon,
            102,
            "dungeon_model_directory",
            "Dungeon geometry source family",
        );
    }
    if has(&["map", "maps", "mapdungeon"]) && !icon {
        add(C::Maps, 120, "map_directory", "Exact map source family");
    }
    if icon {
        add(C::Icons, 122, "icon_directory", "Exact icon artwork family");
    }
    if has(&["bg", "background", "backgrounds"]) && !has(&["mapdungeon", "maps", "map"]) {
        add(
            C::Backgrounds,
            118,
            "background_directory",
            "Background artwork source family",
        );
    }
    if has(&[
        "window",
        "ui",
        "interface",
        "menu",
        "menus",
        "tutorial",
        "book",
        "option",
    ]) {
        add(
            C::Ui,
            100,
            "interface_directory",
            "Interface resource family",
        );
    }
    if stex
        && has(&[
            "battle",
            "camp",
            "common",
            "facility",
            "result",
            "charamake",
            "event",
            "dungeon",
        ])
    {
        add(
            C::Ui,
            72,
            "interface_context",
            "Standalone artwork in an interface family",
        );
    }
    if !game.research_only() && model && !effect {
        if leaf.strip_prefix("en").is_some_and(|s| {
            s.bytes().take_while(u8::is_ascii_digit).count() >= 3
                && s.bytes().all(|b| b.is_ascii_alphanumeric())
        }) || leaf.starts_with("foe_")
        {
            add(
                C::Monsters,
                120,
                "enemy_model_owner",
                "Anchored enemy/FOE model identity",
            );
        }
        if leaf
            .strip_prefix("bf")
            .is_some_and(|s| s.bytes().take_while(u8::is_ascii_digit).count() >= 2)
        {
            add(
                C::Dungeon,
                125,
                "battlefield_model_owner",
                "Anchored battlefield model identity",
            );
        }
    }
    if matches!(game, Game::Eov | Game::Eon) && stex {
        let classes = [
            "fencer",
            "dragoon",
            "warlock",
            "hound",
            "masurao",
            "shaman",
            "herbalist",
            "cestus",
            "reaper",
            "necromancer",
            "earthrun",
            "lunaria",
            "therian",
            "bronie",
            "paladin",
            "ranger",
            "medic",
            "bushido",
            "gunner",
            "prince",
            "shogun",
            "swordman",
            "nightseeker",
            "imperial",
            "brave",
            "doctormags",
            "mystic",
            "farmer",
            "highlander",
            "protector",
            "dancer",
            "pirate",
            "ninja",
            "zodiac",
            "monk",
            "sovereign",
            "landsknecht",
            "arcanist",
            "harbinger",
            "pugilist",
        ];
        if parent.iter().any(|p| {
            p.split_once('_').is_some_and(|(id, name)| {
                id.bytes().all(|b| b.is_ascii_digit())
                    && classes.iter().any(|class| token_family(name, class))
            })
        }) {
            add(
                C::Characters,
                120,
                "class_portrait_owner",
                "Verified player-class portrait resource family",
            );
        }
    }
    if stex
        && has(&[
            "tutorial",
            "cafe",
            "system",
            "parts",
            "form",
            "dungeonname",
            "guildcard",
            "job",
            "skill",
        ])
    {
        add(
            C::Ui,
            100,
            "interface_sheet_family",
            "Interface sheet source family",
        );
    }
    // EO namespace rules are scoped to the five Atlus profiles. An image called
    // "forest_monster" never creates an enemy owner just by containing a word.
    if !game.research_only() {
        if numbered(&logical, "bf") {
            add(
                C::Dungeon,
                124,
                "battlefield_namespace",
                "Anchored bfNN texture namespace",
            );
        }
        if numbered(&logical, "d") && model {
            add(
                C::Dungeon,
                116,
                "dungeon_namespace",
                "Anchored dNN model-texture namespace",
            );
        }
        if logical.starts_with("en_") && model {
            add(
                C::Monsters,
                89,
                "enemy_namespace",
                "Enemy texture namespace; owner evidence preferred",
            );
        }
        if logical.starts_with("ig_")
            && words
                .iter()
                .any(|w| token_family(w, "npc") || token_family(w, "pc"))
        {
            add(
                C::Characters,
                116,
                "portrait_namespace",
                "NPC/player artwork namespace",
            );
        }
        if stex
            && words
                .iter()
                .any(|x| x.starts_with("cha") && x[3..].bytes().all(|b| b.is_ascii_digit()))
        {
            add(
                C::Characters,
                106,
                "character_art_namespace",
                "Character artwork namespace",
            );
        }
        if logical.starts_with("ig_")
            && words.iter().any(|w| token_family(w, "bg"))
            && !has(&["mapdungeon", "map", "maps"])
        {
            add(
                C::Backgrounds,
                112,
                "background_namespace",
                "Standalone background artwork name",
            );
        }
        if logical.starts_with("ig_dmap_") && !icon {
            add(
                C::Maps,
                118,
                "map_namespace",
                "Dungeon-map interface namespace",
            );
        }
        if stex && logical.starts_with("ig_") {
            if words.iter().any(|w| {
                [
                    "txt",
                    "text",
                    "button",
                    "cursor",
                    "floorname",
                    "facilityname",
                    "dungeonname",
                    "info",
                    "number",
                    "num",
                    "menu",
                    "einf",
                    "ptinf",
                    "miss",
                    "burst",
                    "logo",
                ]
                .iter()
                .any(|p| token_family(w, p))
            }) {
                add(
                    C::Ui,
                    106,
                    "interface_art_name",
                    "Anchored interface artwork label",
                );
            }
            if words.iter().any(|w| token_family(w, "effect")) {
                add(
                    C::Effects,
                    106,
                    "effect_art_name",
                    "Standalone effect artwork label",
                );
            }
            // These are known interface families; background/portrait/icon
            // labels above retain their more specific evidence.
            if has(&[
                "top",
                "status",
                "custom",
                "partyinfo",
                "inn",
                "shop",
                "guild",
                "airport",
            ]) {
                add(
                    C::Ui,
                    95,
                    "interface_leaf_family",
                    "Interface screen resource family; review recommended",
                );
            }
        }
    }
    let mut scores = BTreeMap::<C, u16>::new();
    for e in &evidence {
        let v = scores.entry(e.category).or_default();
        *v = (*v).max(e.strength);
    }
    let mut ranked: Vec<_> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let Some(&(best, strength)) = ranked.first() else {
        return Decision {
            category: C::Misc,
            grade: "review".into(),
            needs_review: true,
            evidence,
        };
    };
    let competing = ranked
        .get(1)
        .is_some_and(|(_, s)| *s >= 85 && strength.saturating_sub(*s) < 16);
    let unresolved = competing || strength < 85;
    Decision {
        category: if unresolved { C::Misc } else { best },
        grade: if unresolved {
            "review"
        } else if strength >= 100 {
            "strong"
        } else {
            "inferred"
        }
        .into(),
        needs_review: unresolved || strength < 100,
        evidence,
    }
}

/// Shared pixels retain every ownership context. A confirmed primary choice
/// wins; otherwise contradictory strong origins remain an explicit review.
pub fn for_asset(
    game: Game,
    asset: &crate::catalog::Asset,
    overrides: &crate::catalog::Overrides,
) -> Decision {
    if let Some(choice) = overrides
        .assets
        .get(&asset.id)
        .filter(|o| o.confirmed && o.category.is_some())
    {
        return classify(
            game,
            &asset.source,
            &asset.internal_name,
            &asset.parser,
            Some(choice),
        );
    }
    let mut decisions = Vec::new();
    let mut confirmed = std::collections::BTreeSet::new();
    for origin in &asset.origins {
        let choice = overrides.assets.get(&origin.id);
        if let Some(c) = choice.filter(|o| o.confirmed).and_then(|o| o.category) {
            confirmed.insert(c);
        }
        decisions.push(classify(
            game,
            &origin.source,
            &origin.internal_name,
            &origin.parser,
            choice,
        ));
    }
    if decisions.is_empty() {
        return classify(
            game,
            &asset.source,
            &asset.internal_name,
            &asset.parser,
            None,
        );
    }
    if confirmed.len() == 1 {
        return decisions
            .into_iter()
            .find(|d| d.grade == "confirmed")
            .unwrap();
    }
    let strong: std::collections::BTreeSet<_> = decisions
        .iter()
        .filter(|d| !d.needs_review)
        .map(|d| d.category)
        .collect();
    if confirmed.len() > 1 || strong.len() > 1 {
        return Decision {
            category: C::Misc,
            grade: "shared_review".into(),
            needs_review: true,
            evidence: decisions.into_iter().flat_map(|d| d.evidence).collect(),
        };
    }
    decisions.sort_by_key(|d| {
        (
            d.needs_review,
            std::cmp::Reverse(d.evidence.iter().map(|e| e.strength).max().unwrap_or(0)),
        )
    });
    let mut result = decisions.remove(0);
    result
        .evidence
        .extend(decisions.into_iter().flat_map(|d| d.evidence));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provenance_regressions() {
        let cases = [
            (
                "ROOT/STEX/KEYBOARD/KATAKANA/IG_KEY_KATAKANA045.STEX",
                "",
                "stex",
                C::Fonts,
            ),
            (
                "ROOT/STEX/FACILITY/BG/IG_FAC_BG_05B_L.STEX",
                "",
                "stex",
                C::Backgrounds,
            ),
            (
                "ROOT/EFFECT/ENEMY/SPELL.EPL/0000_P-SMOKE.STEX",
                "",
                "stex",
                C::Effects,
            ),
            ("ROOT/STEX/BOOK/IG_BOO_MONSTER02.STEX", "", "stex", C::Ui),
            (
                "ROOT/MONSTER/MODEL/EN001.BAM",
                "forest_leaf",
                "cgfx",
                C::Monsters,
            ),
            (
                "ROOT/BATTLE/BATTLEFIELD/BF05.BAM",
                "bf05_floor01",
                "cgfx",
                C::Dungeon,
            ),
            (
                "ROOT/STEX/BATTLE/CHARADATA/IG_BAT_CHA10_03.STEX",
                "",
                "stex",
                C::Characters,
            ),
            ("ROOT/STEX/MAPDUNGEON/BG/MAP_BG.STEX", "", "stex", C::Maps),
        ];
        for (source, name, parser, want) in cases {
            let got = classify(Game::Eou, source, name, parser, None);
            assert_eq!(got.category, want, "{source}: {:?}", got.evidence);
            assert!(!got.needs_review, "{source}");
        }
    }
    #[test]
    fn newer_games_use_embedded_paths_and_typed_owner_identity() {
        for game in [Game::Eov, Game::Eon] {
            for (source, name, parser, want) in [
                (
                    "MORI.HPI::EN458B.BAM2",
                    "en_lamia_s_t00",
                    "bch_struct",
                    C::Monsters,
                ),
                (
                    "MORI.HPI::IG_BAT_05MAS07B.STEX",
                    "05_Masurao07/ig_bat_05mas07b.stex",
                    "stex_struct",
                    C::Characters,
                ),
                (
                    "MORI.HPI::SMOKE.EPL::P-SMOKE",
                    "panel/smoke.stex",
                    "stex_struct",
                    C::Effects,
                ),
                (
                    "MORI.HPI::IG_BG.STEX",
                    "Bg_/ig_bg.stex",
                    "stex_struct",
                    C::Backgrounds,
                ),
                (
                    "MORI.HPI::DNAME.STEX",
                    "Dungeonname_/name.stex",
                    "stex_struct",
                    C::Ui,
                ),
                (
                    "MORI.HPI::IG_DMAP_ICON.STEX",
                    "Map/ig_dmap_icon.stex",
                    "stex_struct",
                    C::Icons,
                ),
            ] {
                let decision = classify(game, source, name, parser, None);
                assert_eq!(decision.category, want, "{source}");
                assert!(!decision.needs_review, "{source}");
            }
        }
        assert!(
            classify(
                Game::Eou,
                "unknown.bin",
                "05_Masurao07/art.stex",
                "stex_struct",
                None
            )
            .needs_review
        );
    }
    #[test]
    fn reject_substring_and_conflicting_evidence() {
        assert!(
            classify(
                Game::Eou,
                "unrelated/campfire_enemyish.bin",
                "",
                "stex",
                None
            )
            .needs_review
        );
        let d = classify(
            Game::Eou,
            "MONSTER/MODEL/a.bam",
            "bf05_floor01",
            "cgfx",
            None,
        );
        assert_eq!(d.category, C::Misc);
        assert!(d.needs_review);
    }
    #[test]
    fn confirmed_choice_wins_and_profile_is_scoped() {
        let o = Override {
            category: Some(C::Effects),
            confirmed: true,
            ..Override::default()
        };
        assert_eq!(
            classify(Game::Eou, "MONSTER/a.bam", "bf05_floor01", "cgfx", Some(&o)).category,
            C::Effects
        );
        assert!(classify(Game::Emd, "unknown/a.bin", "bf05_floor01", "cgfx", None).needs_review);
    }
    #[test]
    fn original_battlefield_cases() {
        for name in [
            "bf01_forest1",
            "bf03_water",
            "bf05_ceiling01",
            "bf05_floor01__64x64_f12_421A4257CD1E07B5.png",
        ] {
            assert_eq!(
                classify(Game::Eou, "model.bam", name, "cgfx", None).category,
                C::Dungeon
            );
        }
    }
}
