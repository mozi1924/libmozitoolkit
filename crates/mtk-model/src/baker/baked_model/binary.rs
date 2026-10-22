use super::database::BakedModelDatabase;
use crate::ModelError;

impl BakedModelDatabase {
    /// Serializes and zstd-compresses the database using the compact Postcard format.
    pub fn to_binary(&self) -> Result<Vec<u8>, ModelError> {
        let raw_bytes = postcard::to_allocvec(self)?;
        zstd::encode_all(&raw_bytes[..], 3)
            .map_err(|e| ModelError::Compression(format!("zstd compress error: {e}")))
    }

    /// Deserializes a zstd-compressed Postcard payload into a `BakedModelDatabase`.
    pub fn from_binary(bytes: &[u8]) -> Result<Self, ModelError> {
        const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
        if bytes.len() < 4 || bytes[..4] != ZSTD_MAGIC {
            return Err(ModelError::InvalidDatabasePayload(
                "unsupported model database payload: expected zstd-compressed format \
                 (legacy bincode caches are no longer supported; rebuild the asset cache)"
                    .to_string(),
            ));
        }

        let decompressed = zstd::decode_all(bytes)
            .map_err(|e| ModelError::Compression(format!("zstd decode error: {e}")))?;
        let mut db: BakedModelDatabase = postcard::from_bytes(&decompressed)?;

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
