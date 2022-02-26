use std::{
    env, fs,
    io::{self, Write, BufRead},
    path::{Path, PathBuf}, cmp::min,
};

use indicatif::ProgressBar;
use log::trace;

use crate::Error;

pub fn get_executable_file_name(name: &str) -> Result<String, Error> {
    match env::consts::OS {
        "windows" => Ok(name.to_string() + ".exe"),
        "unix" => Ok(name.into()),
        "linux" => Ok(name.into()),
        _ => Err(Error::UnsupportedOS(env::consts::OS.to_string()))?,
    }
}

pub fn download_file(url: &str, dest: impl AsRef<Path>) -> Result<(), Error> {
    trace!("start downloading file from `{}` to `{:?}`", url,dest.as_ref());
    // let resp = reqwest::blocking::get(url)?.bytes()?;
    // let mut file = std::fs::File::create(dest.as_ref()).map_err(|err| Error::CommonFileError {
    //     message: format!("failed to create file `{:?}`", dest.as_ref()),
    //     source: err,
    // })?;
    // file.write_all(&resp)
    //     .map_err(|err| Error::CommonFileError {
    //         message: format!("failed to write file `{:?}`", dest.as_ref()),
    //         source: err,
    //     })?;

    let resp = reqwest::blocking::Client::new().get(url).send()?;
    let size = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .map(|val| {
            val.to_str()
                .map(|s| s.parse::<u64>().unwrap_or(0))
                .unwrap_or(0)
        })
        .unwrap_or(0);
    if !resp.status().is_success() {
        return Err(Error::NetError(format!(
            "request failed with status: {:?}",
            resp.status()
        )));
    }
    let mut src=io::BufReader::new(resp);
    let mut downloaded=0;
    let bar={
        let pb=ProgressBar::new(size);
        pb
    };

    let mut file = std::fs::File::create(dest.as_ref()).map_err(|err| Error::CommonFileError {
            message: format!("failed to create file `{:?}`", dest.as_ref()),
            source: err,
        })?;

    loop{
        let n={
            let buf=src.fill_buf()?;
            file.write_all(&buf)?;
            buf.len()
        };
        if n==0{
            break;
        }
        src.consume(n);
        downloaded=min(downloaded+n as u64,size);
        bar.set_position(downloaded);
    }
    bar.finish_with_message("finish downloading");

    Ok(())
}

pub fn extract_zip(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<(), Error> {
    let src = src.as_ref();
    let dest = dest.as_ref();
    if !dest.exists() {
        std::fs::create_dir_all(&dest).map_err(|err| Error::CommonFileError {
            message: format!("failed to create directories `{:?}`", &dest),
            source: err,
        })?;
    }
    let mut zip =
        zip::ZipArchive::new(
            std::fs::File::open(&src).map_err(|err| Error::CommonFileError {
                message: format!("failed to open downloaded file `{:?}`", &src),
                source: err,
            })?,
        )?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        if let Some(relative_path) = file.enclosed_name() {
            let out_path = dest.join(relative_path);
            let out_dir = out_path.parent().unwrap();
            if !out_dir.exists() {
                std::fs::create_dir_all(out_dir).map_err(|err| Error::CommonFileError {
                    message: format!("failed to extract directories `{:?}`", &out_dir),
                    source: err,
                })?;
            }
            let mut out_file =
                std::fs::File::create(&out_path).map_err(|err| Error::CommonFileError {
                    message: format!("failed to create extracted file slot `{:?}`", &out_path),
                    source: err,
                })?;
            io::copy(&mut file, &mut out_file).map_err(|err| Error::CommonFileError {
                message: format!("failed to extract file `{:?}`", &out_file),
                source: err,
            })?;
            trace!("unzipped {}", out_path.display());
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
        trace!("process: {:?}", &working_path);

        // Generate a relative path
        let src: PathBuf = working_path.components().skip(input_root).collect();

        // Create a destination if missing
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };
        if fs::metadata(&dest).is_err() {
            trace!(" mkdir: {:?}", dest);
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
                        trace!("  copy: {:?} -> {:?}", &path, &dest_path);
                        fs::copy(&path, &dest_path)?;
                    }
                    None => {
                        trace!("failed: {:?}", path);
                    }
                }
            }
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn test_download_file(){
        let url="https://www.google.com/images/branding/googlelogo/1x/googlelogo_color_272x92dp.png";
        let dest=PathBuf::from("/tmp/googlelogo.png");
        download_file(url,&dest).unwrap();
    }
}