//! Nest configuration file parsing implementation.
//!
//! All the code fetching the settings from the config file is here, in
//! ConfigParser::load_to_config.
//! If settings are added, this is where code must be added to fetch these new settings.
//!
//!

extern crate toml;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::Read;
use std::io;
use std::path::PathBuf;
use std::fmt;

use config::Config;
use repository::{Mirror, Repository};

pub(crate) enum ParseConfError {
    Io(io::Error),
    Deserialize(toml::de::Error),
    Str(String),
}

impl fmt::Display for ParseConfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseConfError::Io(ref err) => write!(f, "{}", err),
            ParseConfError::Deserialize(ref err) => write!(f, "{}", err),
            ParseConfError::Str(ref err) => write!(f, "{}", err),
        }
    }
}

/// A struct holding the TOML main value
#[derive(Debug)]
pub(crate) struct ConfigParser {
    toml: toml::value::Value,
}

impl ConfigParser {
    /// Creates a ConfigParser instance from a TOML file
    ///
    /// self.toml is considered safe to cast to a table after this
    /// ```
    /// self.toml.as_table().unwrap();
    /// ```
    pub(crate) fn new(path: &str) -> Result<ConfigParser, ParseConfError> {
        match ConfigParser::read_conf(path) {
            Ok(conf) => {
                println!("Using {} as config file", path);
                if conf.is_table() {
                    Ok(ConfigParser { toml: conf })
                } else {
                    Err(ParseConfError::Str("Invalid toml file".to_string()))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Replaces the default values in the Config instance with the ones found in the TOML file
    #[inline]
    pub(crate) fn load_to_config(&self, conf: &mut Config) {
        self.parse_paths_mut(conf);
        if let Some(repos) = self.parse_repositories(conf) {
            conf.set_repositories(repos);
        }
    }

    #[inline]
    fn parse_paths_mut(&self, conf: &mut Config) {
        if let Some(paths) = self.get_table("paths") {
            ConfigParser::set_path(conf, Config::set_cache, paths, "cache_dir");
            ConfigParser::set_path(conf, Config::set_download_path, paths, "download_dir");
        }
    }

    #[inline]
    fn set_path<F>(conf: &mut Config, mut func: F, table: &toml::value::Table, key: &str)
    where
        F: FnMut(&mut Config, PathBuf),
    {
        if let Some(string) = ConfigParser::get_str(table, key) {
            func(conf, PathBuf::from(string));
        }
    }

    #[inline]
    fn get_str<'a>(table: &'a toml::value::Table, key: &str) -> Option<&'a str> {
        table.get(key)?.as_str()
    }

    /// Returns a new repository read from the TOML file
    fn parse_repo(
        &self,
        repo_name: &str,
        value: &toml::value::Value,
        conf: &Config,
    ) -> Option<Repository> {
        let mirror_list = value.get("mirrors")?.as_array()?;
        let mut repo = Repository::new(conf, repo_name);
        for mirror in mirror_list {
            repo.mirrors_mut().push(Mirror::new(mirror.as_str()?));
        }
        Some(repo)
    }

    /// Returns a new list of repositories read from the TOML file
    fn parse_repositories(&self, conf: &Config) -> Option<Vec<Repository>> {
        let repositories = self.get_table("repositories")?;
        let mut repo_vec = Vec::with_capacity(repositories.len());
        for (key, value) in repositories {
            if let Some(repo) = self.parse_repo(key, value, conf) {
                repo_vec.push(repo);
            }
        }
        Some(repo_vec)
    }

    #[inline]
    fn get_table(&self, key: &str) -> Option<&toml::value::Table> {
        self.toml
            .as_table()
            .unwrap()
            .get(key)
            .and_then(|value| value.as_table())
    }

    fn read_conf(conf_path: &str) -> Result<toml::Value, ParseConfError> {
        match File::open(conf_path) {
            Ok(file) => {
                let mut file_reader = BufReader::new(file);
                let mut content = String::new();
                if let Err(e) = file_reader.read_to_string(&mut content) {
                    return Err(ParseConfError::Io(e));
                }
                match content.parse::<toml::Value>() {
                    Ok(v) => Ok(v),
                    Err(e) => Err(ParseConfError::Deserialize(e)),
                }
            }
            Err(e) => Err(ParseConfError::Io(e)),
        }
    }
}