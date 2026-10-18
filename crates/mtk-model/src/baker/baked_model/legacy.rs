use std::collections::HashMap;

use super::database::BakedModelDatabase;
use super::model::BakedModel;

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
        let decompressed: Vec<u8>;
        let target_bytes = if bytes.len() >= 4 && bytes[..4] == ZSTD_MAGIC {
            match zstd::decode_all(bytes) {
                Ok(dec) => {
                    decompressed = dec;
                    &decompressed[..]
                }
                Err(e) => {
                    return Err(bincode::Error::new(bincode::ErrorKind::Custom(format!(
                        "zstd decode error: {}",
                        e
                    ))));
                }
            }
        } else {
            bytes
        };

        // 1. Current full format with variant_groups
        if let Ok(mut db) = bincode::deserialize::<BakedModelDatabase>(target_bytes) {
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
            return Ok(db);
        }

        // 2. Legacy models-only HashMap fallback
        if let Ok(mut models) = bincode::deserialize::<HashMap<String, BakedModel>>(target_bytes) {
            for bm in models.values_mut() {
                if bm.culled_faces.iter().all(|v| v.is_empty())
                    && bm.unculled_faces.is_empty()
                    && !bm.elements.is_empty()
                {
                    bm.rebuild_face_buckets();
                }
            }
            let mut variant_groups = HashMap::with_capacity(models.len());
            for (st, bm) in &models {
                variant_groups.insert(
                    st.clone(),
                    super::variant_group::BakedVariantGroup::single(st.clone(), bm.clone()),
                );
            }
            return Ok(Self {
                models,
                variant_groups,
            });
        }

        // 3. Direct deserialization error propagation
        bincode::deserialize(target_bytes)
    }
}
