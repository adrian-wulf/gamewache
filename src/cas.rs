use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::models::CasStats;

#[derive(Clone)]
pub struct CasEngine {
    base_dir: PathBuf,
    db: Database,
}

impl CasEngine {
    pub fn new<P: AsRef<Path>>(storage_dir: P, db: Database) -> io::Result<Self> {
        let base_dir = storage_dir.as_ref().to_path_buf();
        let objects_dir = base_dir.join("objects");
        fs::create_dir_all(&objects_dir)?;

        Ok(Self { base_dir, db })
    }

    pub fn put(&self, data: &[u8], mime: &str, engine: &str) -> io::Result<(String, u64, u64)> {
        let orig_size = data.len() as u64;

        // Blake3 haszowanie (> 5 GB/s)
        let hash = blake3::hash(data).to_hex().to_string();

        let prefix = &hash[0..2];
        let sub_dir = self.base_dir.join("objects").join(prefix);
        fs::create_dir_all(&sub_dir)?;

        let target_path = sub_dir.join(format!("{}.zst", hash));

        let comp_size = if target_path.exists() {
            fs::metadata(&target_path)?.len()
        } else {
            // Kompresja zstd (level 3 dla maksymalnej prędkości transferu)
            let compressed = zstd::encode_all(data, 3)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let size = compressed.len() as u64;

            let tmp_path = sub_dir.join(format!("{}.tmp.{}", hash, uuid::Uuid::new_v4()));
            {
                let mut file = File::create(&tmp_path)?;
                file.write_all(&compressed)?;
                file.flush()?;
            }
            fs::rename(tmp_path, target_path)?;
            size
        };

        // Zapisz metadane w SQLite WAL
        let _ = self.db.record_cas_object(&hash, orig_size, comp_size, mime, engine);

        Ok((hash, orig_size, comp_size))
    }

    pub fn get(&self, hash: &str) -> io::Result<Vec<u8>> {
        if hash.len() < 2 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid hash"));
        }

        let prefix = &hash[0..2];
        let target_path = self
            .base_dir
            .join("objects")
            .join(prefix)
            .join(format!("{}.zst", hash));

        if !target_path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Object not found in CAS"));
        }

        let mut file = File::open(target_path)?;
        let mut compressed = Vec::new();
        file.read_to_end(&mut compressed)?;

        // Dekompresja zstd (> 2500 MB/s)
        zstd::decode_all(&compressed[..])
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    #[allow(dead_code)]
    pub fn has(&self, hash: &str) -> bool {
        if hash.len() < 2 {
            return false;
        }
        let prefix = &hash[0..2];
        self.base_dir
            .join("objects")
            .join(prefix)
            .join(format!("{}.zst", hash))
            .exists()
    }

    pub fn stats(&self) -> io::Result<CasStats> {
        self.db
            .get_cas_stats()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
