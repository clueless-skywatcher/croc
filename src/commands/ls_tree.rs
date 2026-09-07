use clap::Args;
use thiserror::Error;

use crate::commands::Runnable;
use crate::objects::{
    self, CrocError, consume_n_bytes, consume_nul, consume_object_type, consume_str_until, read_int, skip_whitespace,
};

#[derive(Args, Debug, Clone)]
pub struct LsTreeCommand {
    #[arg(long, default_value_t = false)]
    pub name_only: bool,

    pub what: String,
}

#[derive(Error, Debug)]
pub enum LsTreeError {
    #[error("not a tree object (found `{found}`)")]
    NotATree { found: String },

    #[error("failed to decompress object `{hash}`: {reason}")]
    DecompressionFailed { hash: String, reason: String },

    #[error("failed to decode tree object `{hash}`: {reason}")]
    TreeDecodeFailed { hash: String, reason: String },

    #[error("failed to read object `{0}` from disk")]
    IORead(String),

    #[error("invalid object type `{found}` while listing tree `{hash}`")]
    InvalidObjectType { hash: String, found: String },

    #[error("unexpected end of data while decoding tree `{0}`")]
    ReadOutOfBounds(String),
}

struct LsTreeEntry {
    mode: String,
    hash: String,
    obj_name: String,
}

impl Runnable for LsTreeCommand {
    fn run(&self) -> anyhow::Result<()> {
        let decompressed = objects::read_object(&self.what).map_err(|e| self.map_object_error(e))?;
        match self.decode_tree(&decompressed) {
            Ok(s) => {
                println!("{}", s);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl LsTreeCommand {
    fn map_object_error(&self, err: CrocError) -> LsTreeError {
        match err {
            CrocError::IORead => LsTreeError::IORead(self.what.clone()),
            CrocError::DecompressionFailed(reason) => LsTreeError::DecompressionFailed {
                hash: self.what.clone(),
                reason,
            },
            CrocError::DecodeFailed => LsTreeError::TreeDecodeFailed {
                hash: self.what.clone(),
                reason: "malformed object data".to_string(),
            },
            CrocError::InvalidObjectType(found) => LsTreeError::InvalidObjectType {
                hash: self.what.clone(),
                found,
            },
            CrocError::ReadOutOfBounds => LsTreeError::ReadOutOfBounds(self.what.clone()),
        }
    }

    fn decode_failed(&self, reason: impl Into<String>) -> LsTreeError {
        LsTreeError::TreeDecodeFailed {
            hash: self.what.clone(),
            reason: reason.into(),
        }
    }

    fn decode_tree(&self, bytes: &[u8]) -> Result<String, LsTreeError> {
        let mut iter = 0;

        match consume_object_type(bytes, &mut iter) {
            Ok(s) => {
                if s != "tree" {
                    return Err(LsTreeError::NotATree { found: s });
                }
            }
            Err(e) => return Err(self.map_object_error(e)),
        }

        skip_whitespace(bytes, &mut iter);
        
        let mut tree_list: Vec<LsTreeEntry> = Vec::new();

        let Ok(tree_length) = read_int(bytes, &mut iter) else {
            return Err(self.decode_failed("could not parse tree size"));
        };

        while iter < tree_length as usize {
            match self.read_entry(bytes, &mut iter) {
                Ok(entry) => {
                    tree_list.push(entry);
                }
                Err(e) => return Err(e),
            }
        }

        let mut out_str = String::new();

        for entry in tree_list.iter() {
            out_str += &format!("{:<10} {:^10} {:<10}\n", entry.mode, entry.hash, entry.obj_name).to_string();
        }

        Ok(out_str)
    }

    fn read_entry(&self, bytes: &[u8], iter: &mut usize) -> Result<LsTreeEntry, LsTreeError> {
        let Ok(mode_int) = read_int(bytes, iter) else {
            return Err(self.decode_failed("could not parse entry mode"));
        };

        skip_whitespace(bytes, iter);

        let name = match consume_str_until(bytes, iter, b'\0') {
            Ok(s) => s,
            Err(e) => {
                return Err(match e {
                    CrocError::DecodeFailed => self.decode_failed("could not parse entry name"),
                    other => self.map_object_error(other),
                });
            }
        };

        let sha1 = match consume_n_bytes(bytes, iter, 20) {
            Ok(v) => v,
            Err(e) => {
                return Err(match e {
                    CrocError::ReadOutOfBounds => self.decode_failed(format!(
                        "could not read 20-byte hash for entry `{name}`"
                    )),
                    other => self.map_object_error(other),
                });
            }
        };

        Ok(LsTreeEntry {
            mode: mode_int.to_string(),
            hash: hex::encode(sha1),
            obj_name: name,
        })
    }
}
