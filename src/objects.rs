use std::{
    ffi::CStr,
    fs::File,
    io::{Read, SeekFrom::Start},
};

use clap::{ValueEnum, error};
use thiserror::Error;
use zlib_rs::{InflateConfig, ReturnCode, decompress_slice};

#[derive(ValueEnum, Debug, Clone)]
pub enum ObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
}

#[derive(Error, Debug)]
pub enum CrocError {
    #[error("IO error")]
    IORead,

    #[error("Decompression failed: {0:?}")]
    DecompressionFailed(String),

    #[error("Decode failed")]
    DecodeFailed,

    #[error("Invalid object type: {0:?}")]
    InvalidObjectType(String),

    #[error("Out of bounds reading")]
    ReadOutOfBounds,
}

/// Read a stored object by hash and return its zlib-decompressed contents.
pub fn read_object(hash: &str) -> Result<Vec<u8>, CrocError> {
    let prefix = &hash[..2];
    let suffix = &hash[2..];
    let file_path = format!(".croc/objects/{}/{}", prefix, suffix);

    match File::open(file_path) {
        Ok(mut file) => {
            let mut file_contents = vec![];
            if file.read_to_end(&mut file_contents).is_err() {
                return Err(CrocError::IORead);
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
                    return Err(CrocError::DecompressionFailed(reason));
                }
            }

            Ok(decompressed.to_vec())
        }
        Err(_) => {
            panic!("Fatal: file does not exist");
        }
    }
}

/// Read a null-terminated decimal integer from `bytes` starting at `iter`.
pub fn read_int(bytes: &[u8], iter: &mut usize) -> Result<i32, CrocError> {
    let start = *iter;
    while bytes[*iter] != b'\0' && bytes[*iter].is_ascii_digit() {
        *iter += 1;
    }
    
    let Ok(s) = str::from_utf8(&bytes[start..*iter]) else {
        return Err(CrocError::DecodeFailed);
    };

    *iter += 1;
    s.parse::<i32>().map_err(|_| CrocError::DecodeFailed)
}

pub fn skip_whitespace(bytes: &[u8], iter: &mut usize) {
    while bytes[*iter] == b' ' {
        *iter += 1;
    }
}

pub fn consume_nul(bytes: &[u8], iter: &mut usize) {
    assert!(bytes[*iter] == b'\0');
    *iter += 1;
}

pub fn consume_str_until(bytes: &[u8], iter: &mut usize, until: u8) -> Result<String, CrocError> {
    let mut str_bytes: Vec<u8> = vec![];
    while bytes[*iter] != until {
        str_bytes.push(bytes[*iter]);
        *iter += 1;
        if *iter == bytes.len() {
            return Err(CrocError::DecodeFailed);
        }
    }
    
    *iter += 1;

    match str::from_utf8(&str_bytes) {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Err(CrocError::DecodeFailed),
    }
}

pub fn consume_object_type(bytes: &[u8], iter: &mut usize) -> Result<String, CrocError> {
    let start = *iter;
    while bytes[*iter] != b'\0' && bytes[*iter] != b' ' {
        *iter += 1;
    }

    let Ok(s) = str::from_utf8(&bytes[start..*iter]) else {
        return Err(CrocError::DecodeFailed);
    };

    *iter += 1;

    match s {
        "blob" | "tree" | "commit" => Ok(s.to_string()),
        _ => Err(CrocError::InvalidObjectType(s.to_string())),
    }
}

pub fn consume_n_bytes(bytes: &[u8], iter: &mut usize, n: usize) -> Result<Vec<u8>, CrocError> {
    if *iter + n > bytes.len() {
        return Err(CrocError::ReadOutOfBounds);
    }

    let mut out_bytes: Vec<u8> = vec![];
    let mut count = 0;
    while count < n {
        out_bytes.push(bytes[*iter]);
        count += 1;
        *iter += 1;
    }

    Ok(out_bytes)
}
