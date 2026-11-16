//! Constellation name tables and pools, and grouping stars into constellations.

use std::collections::BTreeMap;

use crate::model::{Constellation, LanguageKey, Star};
use crate::rules::{MAX_CONSTELLATIONS, MIN_CONSTELLATION_STARS};

/// First-choice name for each language group.
pub const LANGUAGE_NAMES: &[(LanguageKey, &str)] = &[
    (LanguageKey::Rust, "The Little Crab"),
    (LanguageKey::Python, "The Gentle Serpent"),
    (LanguageKey::Go, "The Gopher"),
    (LanguageKey::JsTs, "The Lantern"),
    (LanguageKey::JavaKotlin, "The Teacup"),
    (LanguageKey::CFamily, "The Anvil"),
    (LanguageKey::HtmlCss, "The Paintbrush"),
    (LanguageKey::Shell, "The Seashell"),
    (LanguageKey::Jupyter, "The Telescope"),
];

/// Fallback names, used for "other" groups and for name clashes.
pub const NAME_POOL: &[&str] = &[
    "The Sleepy Fox",
    "The Tiny Whale",
    "The Curious Owl",
    "The Quiet Moth",
    "The Gentle Hedgehog",
    "The Brave Otter",
    "The Drowsy Bear",
    "The Wandering Deer",
    "The Shy Hare",
    "The Sunny Finch",
    "The Patient Heron",
    "The Wistful Fawn",
    "The Cozy Badger",
    "The Dreaming Cat",
    "The Merry Magpie",
    "The Humble Toad",
    "The Silver Koi",
    "The Sleepy Panda",
    "The Little Lynx",
    "The Careful Crane",
    "The Glowing Firefly",
    "The Mossy Tortoise",
    "The Playful Seal",
    "The Velvet Bat",
    "The Bashful Mole",
    "The Thoughtful Raven",
    "The Dozing Dormouse",
    "The Tender Swan",
    "The Nimble Squirrel",
    "The Starry Jellyfish",
    "The Lazy Sloth",
    "The Busy Bee",
    "The Clever Crow",
    "The Gentle Giraffe",
    "The Rosy Flamingo",
    "The Wise Walrus",
    "The Hopeful Sparrow",
    "The Midnight Lamb",
    "The Cuddly Koala",
    "The Faraway Narwhal",
    "The Peaceful Dove",
    "The Smiling Seahorse",
    "The Dusky Wren",
    "The Waddling Penguin",
    "The Nibbling Rabbit",
    "The Patchwork Pony",
    "The Mellow Newt",
    "The Twilight Dragonfly",
];

/// The first-choice name for a language group, if it has one.
pub fn default_name(key: LanguageKey) -> Option<&'static str> {
    LANGUAGE_NAMES
        .iter()
        .find(|(language, _)| *language == key)
        .map(|(_, name)| *name)
}

/// Choose a constellation name. Fully determined by its inputs (no clock, no
/// unseeded randomness), and never repeats a name in `taken`.
pub fn pick_name(
    key: LanguageKey,
    group_index: usize,
    seed: u64,
    taken: &[String],
    custom: Option<&str>,
) -> String {
    if let Some(name) = custom {
        return name.to_string();
    }
    let is_free = |name: &str| !taken.iter().any(|used| used == name);
    if let Some(name) = default_name(key).filter(|name| is_free(name)) {
        return name.to_string();
    }
    let len = NAME_POOL.len();
    // `as` converts between integer types; the remainder is below `len`, so it fits.
    let start = (seed.wrapping_add(group_index as u64) % len as u64) as usize;
    (0..len)
        .map(|step| NAME_POOL[(start + step) % len])
        .find(|name| is_free(name))
        .unwrap_or(NAME_POOL[start])
        .to_string()
}

/// Group stars by language. Groups of three or more become constellations (the
/// best seven by total score); everything else becomes a wanderer. Sets each
/// star's `constellation` field.
pub fn group_constellations(
    stars: &mut [Star],
    seed: u64,
    custom_names: &BTreeMap<LanguageKey, String>,
) -> (Vec<Constellation>, Vec<usize>) {
    let mut groups: BTreeMap<LanguageKey, Vec<usize>> = BTreeMap::new();
    for (index, star) in stars.iter().enumerate() {
        groups.entry(star.language).or_default().push(index);
    }

    let mut wanderers = Vec::new();
    let mut big: Vec<(LanguageKey, Vec<usize>, f64)> = Vec::new();
    for (key, indices) in groups {
        if indices.len() >= MIN_CONSTELLATION_STARS {
            let total = indices.iter().map(|&i| stars[i].score).sum();
            big.push((key, indices, total));
        } else {
            wanderers.extend(indices);
        }
    }

    // Brightest group first; equal totals fall back to the language order.
    big.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    if big.len() > MAX_CONSTELLATIONS {
        for (_, indices, _) in big.split_off(MAX_CONSTELLATIONS) {
            wanderers.extend(indices);
        }
    }

    let mut constellations: Vec<Constellation> = Vec::new();
    for (group_index, (key, indices, _)) in big.into_iter().enumerate() {
        let taken: Vec<String> = constellations.iter().map(|c| c.name.clone()).collect();
        let custom = custom_names.get(&key).map(String::as_str);
        let name = pick_name(key, group_index, seed, &taken, custom);
        for &index in &indices {
            stars[index].constellation = Some(group_index);
        }
        constellations.push(Constellation {
            name,
            language: key,
            stars: indices,
            edges: Vec::new(),
            anchor: None,
            radius: None,
        });
    }

    wanderers.sort_unstable();
    (constellations, wanderers)
}
