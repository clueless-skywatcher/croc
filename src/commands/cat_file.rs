use std::{
    ffi::CStr,
    fs::File,
    io::Read,
};

use clap::Args;
use thiserror::Error;
use zlib_rs::{InflateConfig, ReturnCode, decompress_slice};

use crate::commands::Runnable;

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
}

impl Runnable for CatFileCommand {
    fn run(&self) -> anyhow::Result<()> {
        let prefix = &self.what[..2];
        let suffix = &self.what[2..];

        let file_path = format!(".croc/objects/{}/{}", prefix, suffix);

        match File::open(file_path.clone()) {
            Ok(mut file) => {
                let mut file_contents = vec![];
                if let Err(_) = file.read_to_end(&mut file_contents) {
                    return Err(CatFileError::IORead.into());
                }
                let mut decompressed_contents = [0u8; 1024];
                let (decompressed, rc) = decompress_slice(
                    &mut decompressed_contents,
                    &file_contents,
                    InflateConfig::default(),
                );

                match rc {
                    ReturnCode::Ok | ReturnCode::StreamEnd | ReturnCode::NeedDict => {}
                    err => {
                        let reason = unsafe { CStr::from_ptr(err.error_message()) }
                            .to_string_lossy()
                            .into_owned();
                        return Err(CatFileError::DecompressionFailed(reason).into());
                    }
                }

                match self.decode_bytes(self.mode.clone(), decompressed) {
                    Ok(s) => {
                        println!("{}", s);
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Err(_) => {
                panic!("Fatal: file does not exist");
            }
        }
    }
}

impl CatFileCommand {
    fn decode_bytes(&self, mode: CatFileMode, bytes: &[u8]) -> Result<String, CatFileError> {
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
                let mut blob_length_str: Vec<u8> = vec![];
                while bytes[iter] != b'\0' {
                    blob_length_str.push(bytes[iter]);
                    iter += 1;
                }

                iter += 1;

                let Ok(blob_length_str) = str::from_utf8(&blob_length_str) else {
                    return Err(CatFileError::DecodeFailed);
                };

                let Ok(blob_length) = blob_length_str.parse::<i32>() else {
                    return Err(CatFileError::DecodeFailed);
                };

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
