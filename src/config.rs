use std::{fs::read_to_string, ops::Deref, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[cfg(feature = "rust")]
    Rust,
    #[cfg(feature = "markdown")]
    Markdown,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    languages: Vec<LanguageConfig>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageConfig {
    pub language: Language,
    pub format: Vec<String>,
    pub query: String,
}

impl Config {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let ctx = || path.display().to_string();

        let s = read_to_string(path).with_context(ctx)?;
        let this: Self = toml::from_str(&s).with_context(ctx)?;

        for v in &this.languages {
            if v.format.is_empty() {
                bail!("format field of language {:?} cannot be empty", v.language);
            }
        }

        Ok(this)
    }
}

impl Language {
    pub fn extension(&self) -> &'static str {
        #[cfg(any(feature = "rust", feature = "markdown"))]
        return match self {
            #[cfg(feature = "rust")]
            Self::Rust => "rs",
            #[cfg(feature = "markdown")]
            Self::Markdown => "md",
        };

        #[allow(unreachable_code)]
        {
            panic!("no language features enabled");
        }
    }
}

impl Deref for Config {
    type Target = [LanguageConfig];

    fn deref(&self) -> &Self::Target {
        &self.languages
    }
}

impl From<Language> for tree_sitter::Language {
    fn from(value: Language) -> Self {
        #[cfg(any(feature = "rust", feature = "markdown"))]
        return match value {
            #[cfg(feature = "rust")]
            Language::Rust => tree_sitter_rust::LANGUAGE,
            #[cfg(feature = "markdown")]
            Language::Markdown => arborium_markdown::language(),
        }
        .into();

        #[allow(unreachable_code)]
        {
            panic!("no language features enabled");
        }
    }
}
