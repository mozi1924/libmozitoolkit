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

/// Copies raw contiguous buffer bytes directly into a destination memory pointer with bounds checking.
pub fn copy_buffer_to_ptr(
    src_bytes: &[u8],
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    let len = src_bytes.len();
    if len == 0 {
        return Ok(0);
    }
    validate_pointer_and_bounds(dst_ptr, len, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), dst_ptr as *mut u8, len);
    }
    Ok(len)
}

/// Copies raw pointer bytes directly into a destination memory pointer with bounds checking.
///
/// # Safety
/// Caller must ensure `src_ptr` points to at least `len` valid, readable bytes.
pub unsafe fn copy_raw_ptr_to_ptr(
    src_ptr: *const u8,
    len: usize,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> DnaResult<usize> {
    if len == 0 {
        return Ok(0);
    }
    if src_ptr.is_null() {
        return Err(DnaError::NullPointer);
    }
    validate_pointer_and_bounds(dst_ptr, len, max_bytes)?;
    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr as *mut u8, len);
    Ok(len)
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
    fn test_copy_buffer_to_ptr() {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        let dst_ptr = dst.as_mut_ptr() as usize;

        let copied = copy_buffer_to_ptr(&src, dst_ptr, Some(5)).unwrap();
        assert_eq!(copied, 5);
        assert_eq!(dst, [1, 2, 3, 4, 5]);

        // Overflow test
        assert!(copy_buffer_to_ptr(&src, dst_ptr, Some(3)).is_err());
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
