use clap::Args;
use thiserror::Error;

use crate::commands::Runnable;
use crate::objects::{self, CrocError};

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
struct CatFileMode {
    #[arg(short = 't')]
    show_type: bool,

    #[arg(short = 'p')]
    show_pretty: bool,

    #[arg(short = 's')]
    show_size: bool,

    #[arg(short = 'e')]
    show_exists: bool,
}

#[derive(Args, Debug, Clone)]
#[command(name = "cat-file")]
pub struct CatFileCommand {
    #[command(flatten)]
    mode: CatFileMode,

    pub what: String,
}

#[derive(Error, Debug)]
pub enum CatFileError {
    #[error("File was not found with this hash: {0:?}")]
    FileNotFound(String),

    #[error("IO error")]
    IORead,

    #[error("Incompatible command options")]
    IncompatibleCommandOptions,

    #[error("Decompression failed: {0:?}")]
    DecompressionFailed(String),

    #[error("Decode failed")]
    DecodeFailed,

    #[error("Invalid object type: {0:?}")]
    InvalidObjectType(String),
}

impl From<CrocError> for CatFileError {
    fn from(err: CrocError) -> Self {
        match err {
            CrocError::IORead => CatFileError::IORead,
            CrocError::DecompressionFailed(reason) => CatFileError::DecompressionFailed(reason),
            CrocError::DecodeFailed => CatFileError::DecodeFailed,
            CrocError::InvalidObjectType(e) => CatFileError::InvalidObjectType(e),
            CrocError::ReadOutOfBounds => CatFileError::DecodeFailed,
        }
    }
}

impl Runnable for CatFileCommand {
    fn run(&self) -> anyhow::Result<()> {
        let decompressed = objects::read_object(&self.what).map_err(CatFileError::from)?;
        match self.decode_bytes(self.mode.clone(), &decompressed) {
            Ok(s) => {
                println!("{}", s);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl CatFileCommand {
    fn decode_bytes(&self, _mode: CatFileMode, bytes: &[u8]) -> Result<String, CatFileError> {
        let mut iter = 0;
        let mut header: Vec<u8> = vec![];

        while bytes[iter] != b' ' {
            header.push(bytes[iter]);
            iter += 1;
        }

        iter += 1;

        let cloned_header = &header.clone();

        let Ok(s) = str::from_utf8(cloned_header) else {
            return Err(CatFileError::DecodeFailed);
        };

        match s {
            "blob" => {
                let blob_length = objects::read_int(bytes, &mut iter)?;

                let mut content_str: Vec<u8> = vec![];
                let mut count = 0;
                while count < blob_length {
                    content_str.push(bytes[iter]);
                    iter += 1;
                    count += 1;
                }

                let Ok(content) = str::from_utf8(&content_str) else {
                    return Err(CatFileError::DecodeFailed);
                };

                Ok(content.to_string())
            }
            _ => Ok("".to_string()),
        }
    }
}
