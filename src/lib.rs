use std::{
    env,
    fs::{self, Permissions},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use checker::{CheckUpdateResult, UpdateChecker};
use data::{PackageTag, VersionTag};
use log::{info, warn};
use thiserror::Error;

pub mod checker;
mod data;
mod utils;

#[derive(Error, Debug)]
pub enum Error {
    #[error("this os `{0}` is not supported")]
    UnsupportedOS(String),
    #[error("invalid local vesion")]
    InvalidLocalVersionError,
    #[error("failed to execute IO operation")]
    UntrackedFileError {
        #[from]
        source: std::io::Error,
    },
    #[error("failed to execute IO operation: {message}")]
    CommonFileError {
        message: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to set permission")]
    PermissionError(#[source] std::io::Error),
    #[error("failed to locate path of current executable")]
    SelfLocationError(#[source] std::io::Error),
    #[error("error when fetching data from the publisher")]
    UntrackedNetError {
        #[from]
        source: reqwest::Error,
    },
    #[error("failed to fetch data from the publisher: {0}")]
    NetError(String),
    #[error("failed to parse data from the publisher")]
    ParsingError {
        #[from]
        source: serde_yaml::Error,
    },
    #[error("failed to extract data")]
    ZipError {
        #[from]
        source: zip::result::ZipError,
    },
}

#[derive(Debug)]
pub struct ReactorBuilder {
    _name: Option<String>,
    _version: Option<data::VersionTag>,
    _publishing_url: Option<String>,
}

impl Default for ReactorBuilder {
    fn default() -> Self {
        Self {
            _name: Default::default(),
            _version: Default::default(),
            _publishing_url: Default::default(),
        }
    }
}

impl ReactorBuilder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self._name = Some(name.into());
        self
    }
    pub fn version(mut self, version: impl TryInto<VersionTag>) -> Self {
        let version = version.try_into();
        match version {
            Ok(version) => self._version = Some(version),
            Err(_) => Err(Error::InvalidLocalVersionError).unwrap(),
        }
        self
    }
    pub fn publishing_url(mut self, publishing_url: impl Into<String>) -> Self {
        self._publishing_url = Some(publishing_url.into());
        self
    }

    pub fn finish(self) -> Reactor {
        Reactor::new(
            self._name.unwrap(),
            self._version.unwrap(),
            self._publishing_url.unwrap(),
        )
    }
}

pub struct Reactor {
    name: String,
    version: data::VersionTag,
    pulishing_url: String,
}

impl Reactor {
    pub fn new(
        name: impl Into<String>,
        version: impl TryInto<VersionTag>,
        pulishing_url: impl Into<String>,
    ) -> Self {
        let version = version.try_into();
        if version.is_err() {
            panic!("invalid version");
        }
        let version = version.unwrap_or_else(|_| VersionTag::new(0, 0, 0));
        let self_name = env::current_exe().unwrap();
        let mut self_version = self_name.file_name().unwrap().to_str().unwrap().split("-");
        if let Some(name_version) = self_version.nth(1) {
            let name_version: VersionTag = name_version.try_into().unwrap();
            if version != name_version {
                panic!("invalid version");
            }
        }
        Reactor {
            name: name.into(),
            version: version,
            pulishing_url: pulishing_url.into(),
        }
    }

    pub fn oneclick(&self) -> Result<(), Error> {
        info!("starting checking update");
        self.self_update_if_available()?;
        self.check_update_and_update()?;
        self.self_update_if_available()?;
        info!("finshed checking update");

        Ok(())
    }

    fn check_update_and_update(&self) -> Result<(), Error> {
        let checker = UpdateChecker::new(self.version, &self.pulishing_url);
        let latest_version = checker.check_update()?;
        if let CheckUpdateResult::UpdateAvailable(latest_version) = latest_version {
            self.update(&latest_version)?;
        } else {
            info!("{} is up to date", self.name);
        }

        Ok(())
    }

    fn self_update_if_available(&self) -> Result<(), Error> {
        thread::sleep(Duration::from_secs(1));

        let mut other_version = self.find_other_available_versions()?;
        other_version.sort();
        // start latest version
        other_version.reverse();
        if let Some(new_version) = other_version.get(0) {
            if new_version.0 > self.version {
                #[cfg(not(windows))]
                {
                    use std::os::unix::prelude::PermissionsExt;
                    fs::set_permissions(&new_version.1, Permissions::from_mode(0o755))
                        .map_err(|err| Error::PermissionError(err))?;
                }
                warn!(
                    "found new local version: {:?}. restarting...",
                    new_version.0
                );
                run_executable_and_quit(new_version.1.canonicalize().unwrap());
            }
        }

        // make self as default executable
        let cur_path = std::env::current_exe().map_err(|err| Error::SelfLocationError(err))?;
        if cur_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .find("-")
            .is_some()
        {
            let new_path = cur_path
                .parent()
                .unwrap()
                .join(utils::get_executable_file_name(&self.name)?);
            fs::copy(&cur_path, &new_path).map_err(|err| Error::CommonFileError {
                message: format!("failed to set current version as default executable"),
                source: err,
            })?;
            warn!("replaced default version. restarting...");
            run_executable_and_quit(new_path.canonicalize().unwrap());
        }

        other_version.sort();
        // remove old versions
        for i in other_version.iter() {
            if i.0 <= self.version {
                fs::remove_file(&i.1).map_err(|err| Error::CommonFileError {
                    message: format!("failed to remove old version `{:?}`", &i.1),
                    source: err,
                })?;
                info!("removed old version: {:?}", i.0);
            }
        }

        Ok(())
    }

    fn find_other_available_versions(&self) -> Result<Vec<(VersionTag, PathBuf)>, Error> {
        let paths = fs::read_dir(".").map_err(|err| Error::CommonFileError {
            message: format!("failed to read current directory"),
            source: err,
        })?;

        let mut result = vec![];
        for path in paths {
            let path = path.map_err(|err| Error::CommonFileError {
                message: format!("failed to read dir"),
                source: err,
            })?;
            let name = path.file_name();
            let name = name.to_str().unwrap();
            if name.starts_with(&self.name) {
                let file_version = name.split("-").nth(1);
                if let Some(file_version) = file_version {
                    let file_version = file_version.try_into();
                    if let Ok(version_tag) = file_version {
                        result.push((version_tag, path.path()));
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn update(&self, package_tag: &PackageTag) -> Result<(), Error> {
        // update lib
        utils::download_file(&package_tag.download_url, "temp.zip")?;
        // TODO: check hash
        let temp_dir = PathBuf::from("./temp");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).map_err(|err| Error::CommonFileError {
                message: format!("failed to clear temp directory `temp`"),
                source: err,
            })?;
        }
        utils::extract_zip("temp.zip", &temp_dir)?;
        info!("extracted remote package");
        utils::copy(&temp_dir, ".").map_err(|err| Error::CommonFileError {
            message: format!("failed to copy directories `{:?}`", &temp_dir),
            source: err,
        })?;
        info!("replaced old data with new data");
        std::fs::remove_dir_all(temp_dir).map_err(|err| Error::CommonFileError {
            message: format!("failed to remove temp directory `temp`"),
            source: err,
        })?;
        std::fs::remove_file("temp.zip").map_err(|err| Error::CommonFileError {
            message: format!("failed to remove temp file `temp.zip`"),
            source: err,
        })?;
        info!("finish file updates");

        Ok(())
    }
}

#[cfg(unix)]
fn run_executable_and_quit(path: impl AsRef<Path>) {
    use std::os::unix::prelude::CommandExt;
    std::process::Command::new(path.as_ref().to_str().unwrap()).exec();
}

#[cfg(windows)]
fn run_executable_and_quit(path: impl AsRef<Path>) {
    std::process::Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path.as_ref().to_str().unwrap())
        .output()
        .unwrap();
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_unzip() {
        let path = PathBuf::from("test.zip");
        let dest = PathBuf::from("./test");
        utils::extract_zip(&path, &dest).unwrap();
        std::fs::remove_dir_all("./test").unwrap();
    }
}
