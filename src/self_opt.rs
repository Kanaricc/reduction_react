use std::{env, path::{Path, PathBuf}, io::Write};

use crate::Error;


const WINDOWS_SCRIPT: &str = r#"
@echo off
timeout /T 3 /NOBREAK >nul
move /Y "{dest}" "{src}"
"#;

const UNIX_SCRIPT: &str = r#"
#!/bin/sh
mv -f "{dest}" "{src}"
"#;

enum ScriptFile {
    Windows(PathBuf),
    Unix(PathBuf),
}

fn create_script(src:impl AsRef<Path>, dest:impl AsRef<Path>) -> Result<ScriptFile,Error> {
    let src = src.as_ref().to_str().unwrap();
    let dest = dest.as_ref().to_str().unwrap();
    let mut template=match env::consts::OS{
        "windows" => WINDOWS_SCRIPT,
        "unix" => UNIX_SCRIPT,
        _ => Err(Error::UnsupportedOS(env::consts::OS.to_string()))?,
    }
    .to_string();
    template = template.replace("{src}", src);
    template = template.replace("{dest}", dest);
    let script = PathBuf::from("./self_opt.sh");
    let mut file = std::fs::File::create(&script)?;
    file.write_all(template.as_bytes())?;

    match env::consts::OS{
        "windows" => Ok(ScriptFile::Windows(script)),
        "unix" => Ok(ScriptFile::Unix(script)),
        _ => Err(Error::UnsupportedOS(env::consts::OS.to_string()))?,
    }
}

fn shutdown_and_replace(src:impl AsRef<Path>, dest:impl AsRef<Path>) -> Result<(),Error> {
    let script = create_script(src, dest)?;
    match script {
        ScriptFile::Windows(script) => {
            std::process::Command::new(script).spawn()?;
        }
        ScriptFile::Unix(script) => {
            std::process::Command::new("sh").arg(script).spawn()?;
        }
    }

    std::process::exit(0);
}

