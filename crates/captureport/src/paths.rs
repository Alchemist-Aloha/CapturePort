use std::{env, fs, io, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> io::Result<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        Ok(Self::from_parts(
            home,
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
        ))
    }

    fn from_parts(
        home: PathBuf,
        config: Option<PathBuf>,
        data: Option<PathBuf>,
        cache: Option<PathBuf>,
    ) -> Self {
        Self {
            config: config
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| home.join(".config"))
                .join("captureport"),
            data: data
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| home.join(".local/share"))
                .join("captureport"),
            cache: cache
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| home.join(".cache"))
                .join("captureport"),
        }
    }

    pub fn create(&self) -> io::Result<()> {
        for path in [&self.config, &self.data, &self.cache] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xdg_defaults_and_overrides_are_distinct() {
        let home = PathBuf::from("/home/example");
        let defaults = AppPaths::from_parts(home.clone(), None, None, None);
        assert_eq!(defaults.config, home.join(".config/captureport"));
        assert_eq!(defaults.data, home.join(".local/share/captureport"));
        assert_eq!(defaults.cache, home.join(".cache/captureport"));
        let custom = AppPaths::from_parts(
            home.clone(),
            Some("/tmp/config".into()),
            Some("/tmp/data".into()),
            Some("relative".into()),
        );
        assert_eq!(custom.config, PathBuf::from("/tmp/config/captureport"));
        assert_eq!(custom.data, PathBuf::from("/tmp/data/captureport"));
        assert_eq!(custom.cache, home.join(".cache/captureport"));
    }
}
