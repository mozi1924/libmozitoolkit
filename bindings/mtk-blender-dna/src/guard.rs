//! Memory bounds validation and safe buffer copy guards.

use crate::error::{DnaError, DnaResult};

/// Validates that `dst_ptr` is non-null and that writing `required_bytes` does not exceed `max_bytes`.
#[inline]
pub fn validate_pointer_and_bounds(
    dst_ptr: usize,
    required_bytes: usize,
    max_bytes: Option<usize>,
) -> DnaResult<()> {
    if dst_ptr == 0 {
        return Err(DnaError::NullPointer);
    }
    if let Some(limit) = max_bytes {
        if required_bytes > limit {
            return Err(DnaError::BufferOverflow {
                required: required_bytes,
                limit,
            });
        }
    }
    Ok(())
}

/// Validates that `dst_ptr` is non-null, aligned to `align` bytes, and that writing `required_bytes` does not exceed `max_bytes`.
#[inline]
pub fn validate_aligned_pointer_and_bounds(
    dst_ptr: usize,
    required_bytes: usize,
    align: usize,
    max_bytes: Option<usize>,
) -> DnaResult<()> {
    validate_pointer_and_bounds(dst_ptr, required_bytes, max_bytes)?;
    if align > 1 && dst_ptr % align != 0 {
        return Err(DnaError::MisalignedPointer {
            ptr: dst_ptr,
            align,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pointer_and_bounds() {
        assert_eq!(
            validate_pointer_and_bounds(0, 10, None),
            Err(DnaError::NullPointer)
        );
        assert_eq!(
            validate_pointer_and_bounds(0x1000, 20, Some(10)),
            Err(DnaError::BufferOverflow {
                required: 20,
                limit: 10
            })
        );
        assert!(validate_pointer_and_bounds(0x1000, 20, Some(20)).is_ok());
        assert!(validate_pointer_and_bounds(0x1000, 20, None).is_ok());
    }

    #[test]
    fn test_validate_aligned_pointer_and_bounds() {
        assert_eq!(
            validate_aligned_pointer_and_bounds(0x1001, 16, 4, None),
            Err(DnaError::MisalignedPointer {
                ptr: 0x1001,
                align: 4
            })
        );
        assert!(validate_aligned_pointer_and_bounds(0x1004, 16, 4, None).is_ok());
    }
}
