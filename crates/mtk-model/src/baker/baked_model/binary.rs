use super::database::BakedModelDatabase;

impl BakedModelDatabase {
    pub fn to_bincode(&self) -> Result<Vec<u8>, bincode::Error> {
        let raw_bytes = bincode::serialize(self)?;
        zstd::encode_all(&raw_bytes[..], 3).map_err(|e| {
            bincode::Error::new(bincode::ErrorKind::Custom(format!(
                "zstd compress error: {}",
                e
            )))
        })
    }

    pub fn from_bincode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
        if bytes.len() < 4 || bytes[..4] != ZSTD_MAGIC {
            return Err(bincode::Error::new(bincode::ErrorKind::Custom(
                "unsupported model database payload: expected zstd-compressed format \
                 (asset cache format < 3 is no longer supported)"
                    .to_string(),
            )));
        }

        let decompressed = zstd::decode_all(bytes).map_err(|e| {
            bincode::Error::new(bincode::ErrorKind::Custom(format!(
                "zstd decode error: {}",
                e
            )))
        })?;
        let mut db: BakedModelDatabase = bincode::deserialize(&decompressed)?;

        for bm in db.models.values_mut() {
            if bm.culled_faces.iter().all(|v| v.is_empty())
                && bm.unculled_faces.is_empty()
                && !bm.elements.is_empty()
            {
                bm.rebuild_face_buckets();
            }
        }
        for vg in db.variant_groups.values_mut() {
            for bm in &mut vg.models {
                if bm.culled_faces.iter().all(|v| v.is_empty())
                    && bm.unculled_faces.is_empty()
                    && !bm.elements.is_empty()
                {
                    bm.rebuild_face_buckets();
                }
            }
        }

        Ok(db)
    }
}
