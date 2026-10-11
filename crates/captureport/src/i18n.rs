//! Offline interface catalogs. Import templates and user data never pass through these catalogs.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum Language {
    #[serde(rename = "zh-Hans")]
    SimplifiedChinese = 1,
    #[serde(rename = "zh-Hant")]
    TraditionalChinese,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "it")]
    Italian,
    #[default]
    #[serde(rename = "en", other)]
    English = 0,
}

impl Language {
    pub(crate) const ALL: [Self; 10] = [
        Self::English,
        Self::SimplifiedChinese,
        Self::TraditionalChinese,
        Self::Spanish,
        Self::French,
        Self::German,
        Self::Portuguese,
        Self::Japanese,
        Self::Korean,
        Self::Italian,
    ];

    pub(crate) fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
            Self::TraditionalChinese => "繁體中文",
            Self::Spanish => "Español",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Portuguese => "Português",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Italian => "Italiano",
        }
    }

    fn catalog(self) -> &'static str {
        match self {
            Self::English => include_str!("../assets/locales/en.json"),
            Self::SimplifiedChinese => include_str!("../assets/locales/zh-Hans.json"),
            Self::TraditionalChinese => include_str!("../assets/locales/zh-Hant.json"),
            Self::Spanish => include_str!("../assets/locales/es.json"),
            Self::French => include_str!("../assets/locales/fr.json"),
            Self::German => include_str!("../assets/locales/de.json"),
            Self::Portuguese => include_str!("../assets/locales/pt.json"),
            Self::Japanese => include_str!("../assets/locales/ja.json"),
            Self::Korean => include_str!("../assets/locales/ko.json"),
            Self::Italian => include_str!("../assets/locales/it.json"),
        }
    }

    fn from_locale(locale: &str) -> Self {
        let locale = locale.to_ascii_lowercase().replace('_', "-");
        let locale = locale.split(['.', '@']).next().unwrap_or_default();
        let parts: Vec<_> = locale.split('-').collect();
        match parts.first().copied().unwrap_or_default() {
            "zh" if parts
                .iter()
                .any(|part| matches!(*part, "hant" | "tw" | "hk" | "mo")) =>
            {
                Self::TraditionalChinese
            }
            "zh" => Self::SimplifiedChinese,
            "es" => Self::Spanish,
            "fr" => Self::French,
            "de" => Self::German,
            "pt" => Self::Portuguese,
            "ja" => Self::Japanese,
            "ko" => Self::Korean,
            "it" => Self::Italian,
            _ => Self::English,
        }
    }

    pub(crate) fn system() -> Self {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()))
            .map(|locale| Self::from_locale(&locale))
            .unwrap_or_default()
    }
}

static ACTIVE: AtomicU8 = AtomicU8::new(0);
static CATALOGS: OnceLock<Vec<BTreeMap<String, String>>> = OnceLock::new();

pub(crate) fn activate(language: Option<Language>) {
    ACTIVE.store(
        language.unwrap_or_else(Language::system) as u8,
        Ordering::Relaxed,
    );
}

fn lookup(language: Language, key: &str) -> &str {
    let catalogs = CATALOGS.get_or_init(|| {
        Language::ALL
            .iter()
            .map(|language| {
                serde_json::from_str(language.catalog()).expect("valid bundled translation catalog")
            })
            .collect()
    });
    catalogs[language as usize]
        .get(key)
        .map(String::as_str)
        .unwrap_or(key)
}

pub(crate) fn tr(key: &str) -> String {
    let language = Language::ALL[ACTIVE.load(Ordering::Relaxed) as usize];
    lookup(language, key).to_owned()
}

/// Substitute once, so filenames containing `{0}` cannot be interpreted as another argument.
fn interpolate(template: &str, values: &[String]) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find('}')
            && let Ok(index) = rest[1..end].parse::<usize>()
            && let Some(value) = values.get(index)
        {
            result.push_str(value);
            rest = &rest[end + 1..];
        } else {
            result.push('{');
            rest = &rest[1..];
        }
    }
    result.push_str(rest);
    result
}

pub(crate) fn format(key: &str, values: &[String]) -> String {
    interpolate(&tr(key), values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(value: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut rest = value;
        while let Some(start) = rest.find('{') {
            let Some(end) = rest[start..].find('}') else {
                panic!("unclosed placeholder: {value}");
            };
            placeholders.push(rest[start..=start + end].to_owned());
            rest = &rest[start + end + 1..];
        }
        placeholders.sort();
        placeholders
    }

    #[test]
    fn all_catalogs_cover_english_and_preserve_placeholders() {
        let english: BTreeMap<String, String> =
            serde_json::from_str(Language::English.catalog()).unwrap();
        assert_eq!(Language::ALL.len(), 10);
        for (index, language) in Language::ALL.into_iter().enumerate() {
            assert_eq!(language as usize, index);
            let catalog: BTreeMap<String, String> =
                serde_json::from_str(language.catalog()).unwrap();
            assert_eq!(
                catalog.keys().collect::<Vec<_>>(),
                english.keys().collect::<Vec<_>>(),
                "{language:?}"
            );
            for (key, value) in catalog {
                assert!(!value.trim().is_empty(), "{language:?}: {key}");
                assert_eq!(
                    placeholders(&key),
                    placeholders(&value),
                    "{language:?}: {key}"
                );
            }
        }
        assert_eq!(lookup(Language::SimplifiedChinese, "Settings"), "设置");
        assert_eq!(lookup(Language::TraditionalChinese, "Settings"), "設定");
        assert_eq!(
            lookup(Language::German, "unknown future key"),
            "unknown future key"
        );
    }

    #[test]
    fn interface_calls_and_template_help_have_catalog_entries() {
        let english: BTreeMap<String, String> =
            serde_json::from_str(Language::English.catalog()).unwrap();
        for source in [
            include_str!("main.rs"),
            include_str!("model.rs"),
            include_str!("render.rs"),
            include_str!("tasks.rs"),
            include_str!("browser/detail.rs"),
            include_str!("browser/gallery.rs"),
            include_str!("browser/import.rs"),
            include_str!("browser/panels.rs"),
            include_str!("browser/render.rs"),
            include_str!("browser/session.rs"),
            include_str!("browser/settings.rs"),
            include_str!("browser/sources.rs"),
            include_str!("browser/views.rs"),
        ] {
            for call in ["i18n::tr(\"", "i18n::format(\""] {
                for rest in source.split(call).skip(1) {
                    let mut escaped = false;
                    let end = rest
                        .char_indices()
                        .find_map(|(index, ch)| {
                            if escaped {
                                escaped = false;
                            } else if ch == '\\' {
                                escaped = true;
                            } else if ch == '"' {
                                return Some(index);
                            }
                            None
                        })
                        .unwrap();
                    let key: String =
                        serde_json::from_str(&format!("\"{}\"", &rest[..end])).unwrap();
                    assert!(english.contains_key(&key), "missing interface key: {key}");
                }
            }
        }
        for token in captureport_ingest::TEMPLATE_TOKENS {
            assert!(english.contains_key(token.group), "{}", token.group);
            assert!(
                english.contains_key(token.description),
                "{}",
                token.description
            );
        }
        assert!(
            english
                .keys()
                .any(|key| key.starts_with("Use text and {segments}"))
        );
        assert!(
            english
                .keys()
                .any(|key| key.starts_with("Use / between folders"))
        );
    }

    #[test]
    fn locales_support_regions_scripts_and_posix_fallback() {
        for locale in ["zh_CN.UTF-8", "zh-SG", "zh-Hans"] {
            assert_eq!(Language::from_locale(locale), Language::SimplifiedChinese);
        }
        for locale in ["zh_TW.UTF-8", "zh-Hant", "zh_HK", "zh-MO"] {
            assert_eq!(Language::from_locale(locale), Language::TraditionalChinese);
        }
        assert_eq!(Language::from_locale("pt_BR.UTF-8"), Language::Portuguese);
        assert_eq!(Language::from_locale("de_DE.UTF-8"), Language::German);
        assert_eq!(Language::from_locale("C.UTF-8"), Language::English);
        assert_eq!(Language::from_locale("ar_EG.UTF-8"), Language::English);
        assert_eq!(
            serde_json::from_str::<Language>("\"future-language\"").unwrap(),
            Language::English
        );
    }

    #[test]
    fn formatting_reorders_arguments_without_reinterpreting_user_data() {
        assert_eq!(
            interpolate("{1}: {0} / {1}", &["图像{1}.jpg".into(), "2".into()]),
            "2: 图像{1}.jpg / 2"
        );
        assert_eq!(
            interpolate("{0} · {1}", &["/media/繁體中文/IMG.JPG".into(), "1".into()]),
            "/media/繁體中文/IMG.JPG · 1"
        );
    }
}
