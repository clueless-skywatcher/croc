use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use clap::{Args, ValueEnum};
use sha1::{Digest, Sha1};
use thiserror::Error;

use crate::commands::Runnable;

#[derive(Args, Debug, Clone)]
pub struct HashObjectCommand {
    #[arg(short = 't', long = "type", default_value = "blob")]
    pub kind: HashObjectType,

    #[arg(short = 'w', default_value_t = false)]
    pub write: bool,

    pub file: PathBuf,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum HashObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
}

#[derive(Error, Debug)]
pub enum HashObjectError {
    #[error("file does not exist: {0:?}")]
    FileDoesNotExist(PathBuf),

    #[error("failed to read {path:?}")]
    Io { path: PathBuf, source: io::Error },

    #[error("hashing {0:?} objects is not implemented")]
    Unimplemented(HashObjectType),
}

impl Runnable for HashObjectCommand {
    fn run(&self) -> anyhow::Result<()> {
        let contents = match self.kind {
            HashObjectType::Blob => self.read_file()?,
            ref kind => return Err(HashObjectError::Unimplemented(kind.clone()).into()),
        };

        let mut final_contents: Vec<u8> = Vec::new();

        final_contents.extend_from_slice(b"blob ");
        final_contents.extend_from_slice(&contents.len().to_string().as_bytes());
        final_contents.push(0u8);
        final_contents.extend_from_slice(&contents);

        let hash = Sha1::digest(final_contents.clone());
        let encoded = hex::encode(hash);

        if self.write {
            let prefix = &encoded[..2];
            let suffix = &encoded[2..];
            let dir_path = PathBuf::from(format!(".croc/objects/{}", prefix));

            match fs::create_dir_all(dir_path.clone()) {
                Ok(_) => match File::create(dir_path.join(suffix)) {
                    Ok(mut file) => match file.write(&final_contents) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(HashObjectError::Io {
                                path: dir_path.join(suffix),
                                source: e,
                            }
                            .into());
                        }
                    },
                    Err(e) => {
                        return Err(HashObjectError::Io {
                            path: dir_path.join(suffix),
                            source: e,
                        }
                        .into());
                    }
                },
                Err(e) => {
                    return Err(HashObjectError::Io {
                        path: dir_path.join(suffix),
                        source: e,
                    }
                    .into());
                }
            }
        }

        println!("{}", encoded);

        Ok(())
    }
}

impl HashObjectCommand {
    fn read_file(&self) -> Result<Vec<u8>, HashObjectError> {
        fs::read(&self.file).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => HashObjectError::FileDoesNotExist(self.file.clone()),
            _ => HashObjectError::Io {
                path: self.file.clone(),
                source: e,
            },
        })
    }
}
