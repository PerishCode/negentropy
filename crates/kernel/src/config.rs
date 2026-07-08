use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Config {
    pub file: File,
    pub vocab: Vec<String>,
}

pub fn load(root: &Path) -> Config {
    Config {
        file: file(root.join("negentropy.toml")),
        vocab: vocab(root.join("vocabulary.toml")),
    }
}

fn file(path: PathBuf) -> File {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    toml::from_str(&text).unwrap_or_default()
}

fn vocab(path: PathBuf) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let store: Store = toml::from_str(&text).unwrap_or_default();
    store
        .compound
        .into_iter()
        .filter(|(_, why)| !why.trim().is_empty())
        .map(|(name, _)| name)
        .collect()
}

#[derive(Deserialize, Default)]
struct Store {
    #[serde(default)]
    compound: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
pub struct File {
    #[serde(default)]
    pub limit: Limit,
    #[serde(default)]
    pub comment: Comment,
    #[serde(default)]
    pub word: Word,
    #[serde(default)]
    pub boundary: Vec<Boundary>,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Limit {
    pub block: usize,
    pub path: usize,
}

impl Default for Limit {
    fn default() -> Self {
        Self { block: 3, path: 3 }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Comment {
    pub allow: bool,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Word {
    pub single: bool,
}

impl Default for Word {
    fn default() -> Self {
        Self { single: true }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Boundary {
    pub paths: Vec<String>,
    pub allow: Vec<String>,
}

impl Config {
    pub fn exempt(&self, path: &str, law: &str) -> bool {
        self.file
            .boundary
            .iter()
            .any(|edge| covers(edge, path, law))
    }
}

fn covers(edge: &Boundary, path: &str, law: &str) -> bool {
    edge.allow.iter().any(|name| name == law) && edge.paths.iter().any(|glob| prefix(glob, path))
}

fn prefix(glob: &str, path: &str) -> bool {
    let head = glob.trim_end_matches('*').trim_end_matches('/');
    head.is_empty() || path.starts_with(head)
}
