use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf}, env,
};

use crate::Error;

pub fn get_executable_file_name(name:&str)->Result<String,Error> {
    match env::consts::OS {
        "windows" => Ok(name.to_string()+".exe"),
        "unix" => Ok(name.into()),
        "linux"=>Ok(name.into()),
        _=>Err(Error::UnsupportedOS(env::consts::OS.to_string()))?,
    }
}

pub fn download_file(url: &str, dest: impl AsRef<Path>) -> Result<(), Error> {
    let resp = reqwest::blocking::get(url)?.bytes()?;
    let mut file = std::fs::File::create(dest.as_ref())?;
    file.write_all(&resp)?;
    Ok(())
}

pub fn extract_zip(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<(), Error> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    if !dest.exists() {
        std::fs::create_dir_all(&dest)?;
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&src)?)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        if let Some(relative_path) = file.enclosed_name() {
            let out_path = dest.join(relative_path);
            println!("{}", out_path.display());
            let out_dir = out_path.parent().unwrap();
            if !out_dir.exists() {
                std::fs::create_dir_all(out_dir)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            io::copy(&mut file, &mut out_file)?;
        }
    }

    Ok(())
}

pub fn copy<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> Result<(), std::io::Error> {
    let mut stack = Vec::new();
    stack.push(PathBuf::from(from.as_ref()));

    let output_root = PathBuf::from(to.as_ref());
    let input_root = PathBuf::from(from.as_ref()).components().count();

    while let Some(working_path) = stack.pop() {
        // println!("process: {:?}", &working_path);

        // Generate a relative path
        let src: PathBuf = working_path.components().skip(input_root).collect();

        // Create a destination if missing
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };
        if fs::metadata(&dest).is_err() {
            // println!(" mkdir: {:?}", dest);
            fs::create_dir_all(&dest)?;
        }

        for entry in fs::read_dir(working_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => {
                        let dest_path = dest.join(filename);
                        // println!("  copy: {:?} -> {:?}", &path, &dest_path);
                        fs::copy(&path, &dest_path)?;
                    }
                    None => {
                        // println!("failed: {:?}", path);
                    }
                }
            }
        }
    }

    Ok(())
}
