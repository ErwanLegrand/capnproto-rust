//! SIMD acceleration for Cap'n Proto
//!
//! This module provides SIMD-optimized functions for serialization and deserialization.
//! It uses Rust's portable SIMD when available, with fallback to scalar implementations.



/// Detect available SIMD features at runtime
pub fn detect_simd_features() -> SimdFeatures {
    SimdFeatures {
        avx2: is_x86_feature_detected!("avx2"),
        sse2: is_x86_feature_detected!("sse2"),
        neon: cfg!(target_arch = "aarch64"),
    }
}

/// Available SIMD features
#[derive(Debug, Clone, Copy)]
pub struct SimdFeatures {
    pub avx2: bool,
    pub sse2: bool,
    pub neon: bool,
}

/// NEON implementation for ARM
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn write_u32_le_neon(output: &mut [u8], values: &[u32]) {
    use std::arch::aarch64::*;
    
    let mut i = 0;
    let len = values.len().min(output.len() / 4);
    
    // Process 4 values at a time using NEON
    while i + 4 <= len {
        let chunk = vld1q_u32(values.as_ptr().add(i));
        
        // Store the values directly (they're already in little-endian in memory)
        vst1q_u8(output.as_mut_ptr().add(i * 4) as *mut u8, vreinterpretq_u8_u32(chunk));
        
        i += 4;
    }
    
    // Process remaining values with scalar
    while i < len {
        let value = values[i];
        output[i * 4] = value as u8;
        output[i * 4 + 1] = (value >> 8) as u8;
        output[i * 4 + 2] = (value >> 16) as u8;
        output[i * 4 + 3] = (value >> 24) as u8;
        i += 1;
    }
}

/// Convert u32 values to bytes in little-endian format using SIMD
#[inline]
pub fn write_u32_le_simd(output: &mut [u8], values: &[u32]) {
    #[cfg(target_feature = "avx2")]
    if is_x86_feature_detected!("avx2") {
        return unsafe { write_u32_le_avx2(output, values) };
    }
    
    #[cfg(target_feature = "sse2")]
    if is_x86_feature_detected!("sse2") {
        return unsafe { write_u32_le_sse2(output, values) };
    }
    
    #[cfg(target_arch = "aarch64")]
    if cfg!(target_feature = "neon") {
        return unsafe { write_u32_le_neon(output, values) };
    }
    
    // Fallback to scalar implementation
    write_u32_le_scalar(output, values);
}

/// Scalar fallback implementation
#[inline]
fn write_u32_le_scalar(output: &mut [u8], values: &[u32]) {
    for (i, &value) in values.iter().enumerate() {
        if i * 4 + 3 < output.len() {
            output[i * 4] = value as u8;
            output[i * 4 + 1] = (value >> 8) as u8;
            output[i * 4 + 2] = (value >> 16) as u8;
            output[i * 4 + 3] = (value >> 24) as u8;
        }
    }
}

/// AVX2 implementation for x86_64
#[cfg(target_feature = "avx2")]
#[target_feature(enable = "avx2")]
unsafe fn write_u32_le_avx2(output: &mut [u8], values: &[u32]) {
    use std::arch::x86_64::*;
    
    let mut i = 0;
    let len = values.len().min(output.len() / 4);
    
    // Process 8 values at a time using AVX2
    while i + 8 <= len {
        let chunk = _mm256_loadu_si256(values.as_ptr().add(i) as *const __m256i);
        
        // Store the values directly (they're already in little-endian in memory)
        _mm256_storeu_si256(output.as_mut_ptr().add(i * 4) as *mut __m256i, chunk);
        
        i += 8;
    }
    
    // Process remaining values with scalar
    while i < len {
        let value = values[i];
        output[i * 4] = value as u8;
        output[i * 4 + 1] = (value >> 8) as u8;
        output[i * 4 + 2] = (value >> 16) as u8;
        output[i * 4 + 3] = (value >> 24) as u8;
        i += 1;
    }
}

/// SSE2 implementation for x86_64
#[cfg(target_feature = "sse2")]
#[target_feature(enable = "sse2")]
unsafe fn write_u32_le_sse2(output: &mut [u8], values: &[u32]) {
    use std::arch::x86_64::*;
    
    let mut i = 0;
    let len = values.len().min(output.len() / 4);
    
    // Process 4 values at a time using SSE2
    while i + 4 <= len {
        let chunk = _mm_loadu_si128(values.as_ptr().add(i) as *const __m128i);
        
        // Store the values directly (they're already in little-endian in memory)
        _mm_storeu_si128(output.as_mut_ptr().add(i * 4) as *mut __m128i, chunk);
        
        i += 4;
    }
    
    // Process remaining values with scalar
    while i < len {
        let value = values[i];
        output[i * 4] = value as u8;
        output[i * 4 + 1] = (value >> 8) as u8;
        output[i * 4 + 2] = (value >> 16) as u8;
        output[i * 4 + 3] = (value >> 24) as u8;
        i += 1;
    }
}

/// Optimized segment table writing using SIMD
pub fn write_segment_table_simd(
    output: &mut [u8],
    segment_counts: &[u32],
    segment_sizes: &[u32]
) -> usize {
    let mut pos = 0;
    
    // Write segment count (first word)
    if pos + 4 <= output.len() && !segment_counts.is_empty() {
        let count = segment_counts[0].saturating_sub(1);
        output[pos] = count as u8;
        output[pos + 1] = (count >> 8) as u8;
        output[pos + 2] = (count >> 16) as u8;
        output[pos + 3] = (count >> 24) as u8;
        pos += 4;
    }
    
    // Write first segment size
    if pos + 4 <= output.len() && !segment_sizes.is_empty() {
        let size = segment_sizes[0];
        output[pos] = size as u8;
        output[pos + 1] = (size >> 8) as u8;
        output[pos + 2] = (size >> 16) as u8;
        output[pos + 3] = (size >> 24) as u8;
        pos += 4;
    }
    
    // Write remaining segment sizes using SIMD
    if segment_sizes.len() > 1 {
        let remaining_sizes = &segment_sizes[1..];
        let output_slice = &mut output[pos..];
        
        // Convert u32 sizes to bytes
        let mut byte_buffer = vec![0u8; remaining_sizes.len() * 4];
        write_u32_le_simd(&mut byte_buffer, remaining_sizes);
        
        // Copy to output
        let copy_len = byte_buffer.len().min(output_slice.len());
        output_slice[..copy_len].copy_from_slice(&byte_buffer[..copy_len]);
        pos += copy_len;
    }
    
    pos
}

/// Optimized segment writing using SIMD for bulk operations
pub fn write_segments_simd(output: &mut [u8], segments: &[&[u8]]) -> usize {
    let mut pos = 0;
    
    for segment in segments {
        let copy_len = segment.len().min(output.len() - pos);
        if copy_len > 0 {
            // Use SIMD-optimized copy when possible
            #[cfg(target_feature = "avx2")]
            if is_x86_feature_detected!("avx2") && copy_len >= 32 {
                unsafe { 
                    use std::arch::x86_64::*;
                    let src = segment.as_ptr();
                    let dst = output.as_mut_ptr().add(pos);
                    let mut i = 0;
                    
                    // Process 32 bytes at a time using AVX2
                    while i + 32 <= copy_len {
                        let chunk1 = _mm256_loadu_si256(src.add(i) as *const __m256i);
                        let chunk2 = _mm256_loadu_si256(src.add(i + 32) as *const __m256i);
                        _mm256_storeu_si256(dst.add(i) as *mut __m256i, chunk1);
                        _mm256_storeu_si256(dst.add(i + 32) as *mut __m256i, chunk2);
                        i += 64;
                    }
                    
                    // Process remaining bytes
                    if i < copy_len {
                        std::ptr::copy_nonoverlapping(src.add(i), dst.add(i), copy_len - i);
                    }
                }
            } else if copy_len >= 16 {
                // Use 16-byte aligned copies when possible
                let src = segment.as_ptr();
                let dst = output.as_mut_ptr().add(pos);
                let mut i = 0;
                
                // Process 16 bytes at a time
                while i + 16 <= copy_len {
                    unsafe {
                        std::ptr::copy_nonoverlapping(src.add(i), dst.add(i), 16);
                    }
                    i += 16;
                }
                
                // Process remaining bytes
                if i < copy_len {
                    unsafe {
                        std::ptr::copy_nonoverlapping(src.add(i), dst.add(i), copy_len - i);
                    }
                }
            } else {
                output[pos..pos + copy_len].copy_from_slice(&segment[..copy_len]);
            }
            pos += copy_len;
        }
    }
    
    pos
}

/// Read u32 values from little-endian bytes using SIMD
#[inline]
pub fn read_u32_le_simd(input: &[u8], output: &mut [u32]) {
    #[cfg(target_feature = "avx2")]
    if is_x86_feature_detected!("avx2") {
        return unsafe { read_u32_le_avx2(input, output) };
    }
    
    #[cfg(target_feature = "sse2")]
    if is_x86_feature_detected!("sse2") {
        return unsafe { read_u32_le_sse2(input, output) };
    }
    
    #[cfg(target_arch = "aarch64")]
    if cfg!(target_feature = "neon") {
        return unsafe { read_u32_le_neon(input, output) };
    }
    
    // Fallback to scalar implementation
    read_u32_le_scalar(input, output);
}

/// Scalar fallback implementation for reading u32 from LE bytes
#[inline]
fn read_u32_le_scalar(input: &[u8], output: &mut [u32]) {
    let len = output.len().min(input.len() / 4);
    for i in 0..len {
        let offset = i * 4;
        let bytes = [input[offset], input[offset + 1], input[offset + 2], input[offset + 3]];
        output[i] = u32::from_le_bytes(bytes);
    }
}

/// AVX2 implementation for reading u32 from LE bytes
#[cfg(target_feature = "avx2")]
#[target_feature(enable = "avx2")]
unsafe fn read_u32_le_avx2(input: &[u8], output: &mut [u32]) {
    use std::arch::x86_64::*;
    
    let len = output.len().min(input.len() / 4);
    let mut i = 0;
    
    // Process 8 values at a time using AVX2
    while i + 8 <= len {
        let chunk = _mm256_loadu_si256(input.as_ptr().add(i * 4) as *const __m256i);
        _mm256_storeu_si256(output.as_mut_ptr().add(i) as *mut __m256i, chunk);
        i += 8;
    }
    
    // Process remaining values with scalar
    while i < len {
        let offset = i * 4;
        let bytes = [input[offset], input[offset + 1], input[offset + 2], input[offset + 3]];
        output[i] = u32::from_le_bytes(bytes);
        i += 1;
    }
}

/// SSE2 implementation for reading u32 from LE bytes
#[cfg(target_feature = "sse2")]
#[target_feature(enable = "sse2")]
unsafe fn read_u32_le_sse2(input: &[u8], output: &mut [u32]) {
    use std::arch::x86_64::*;
    
    let len = output.len().min(input.len() / 4);
    let mut i = 0;
    
    // Process 4 values at a time using SSE2
    while i + 4 <= len {
        let chunk = _mm_loadu_si128(input.as_ptr().add(i * 4) as *const __m128i);
        _mm_storeu_si128(output.as_mut_ptr().add(i) as *mut __m128i, chunk);
        i += 4;
    }
    
    // Process remaining values with scalar
    while i < len {
        let offset = i * 4;
        let bytes = [input[offset], input[offset + 1], input[offset + 2], input[offset + 3]];
        output[i] = u32::from_le_bytes(bytes);
        i += 1;
    }
}

/// NEON implementation for reading u32 from LE bytes
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn read_u32_le_neon(input: &[u8], output: &mut [u32]) {
    use std::arch::aarch64::*;
    
    let len = output.len().min(input.len() / 4);
    let mut i = 0;
    
    // Process 4 values at a time using NEON
    while i + 4 <= len {
        let chunk = vld1q_u8(input.as_ptr().add(i * 4));
        let int_chunk = vreinterpretq_u32_u8(chunk);
        vst1q_u32(output.as_mut_ptr().add(i), int_chunk);
        i += 4;
    }
    
    // Process remaining values with scalar
    while i < len {
        let offset = i * 4;
        let bytes = [input[offset], input[offset + 1], input[offset + 2], input[offset + 3]];
        output[i] = u32::from_le_bytes(bytes);
        i += 1;
    }
}

/// Validates that no bytes in the data are zero (null bytes) using SIMD
#[inline]
pub fn validate_no_null_bytes_simd(data: &[u8]) -> bool {
    let len = data.len();
    let mut i = 0;
    
    // Process 16 bytes at a time using SIMD when possible
    #[cfg(target_feature = "avx2")]
    if is_x86_feature_detected!("avx2") && len >= 16 {
        use std::arch::x86_64::*;
        
        while i + 16 <= len {
            let chunk = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);
            let zero_mask = _mm256_cmpeq_epi8(chunk, _mm256_setzero_si256());
            let mask = _mm256_movemask_epi8(zero_mask);
            
            if mask != 0 {
                return false; // Found null byte
            }
            
            i += 16;
        }
    }
    
    // Process remaining bytes
    while i < len {
        if data[i] == 0 {
            return false;
        }
        i += 1;
    }
    
    true
}

/// Validates that all bytes in the data are within a specific range using SIMD
#[inline]
pub fn validate_range_simd(data: &[u8], min: u8, max: u8) -> bool {
    let len = data.len();
    let mut i = 0;
    
    // Process 16 bytes at a time using SIMD when possible
    #[cfg(target_feature = "avx2")]
    if is_x86_feature_detected!("avx2") && len >= 16 {
        use std::arch::x86_64::*;
        
        let min_vec = _mm256_set1_epi8(min as i8);
        let max_vec = _mm256_set1_epi8(max as i8);
        
        while i + 16 <= len {
            let chunk = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);
            
            // Check if any byte is less than min
            let under_mask = _mm256_cmpgt_epi8(min_vec, chunk);
            // Check if any byte is greater than max
            let over_mask = _mm256_cmpgt_epi8(chunk, max_vec);
            
            let under_result = _mm256_movemask_epi8(under_mask);
            let over_result = _mm256_movemask_epi8(over_mask);
            
            if under_result != 0 || over_result != 0 {
                return false; // Found out-of-range byte
            }
            
            i += 16;
        }
    }
    
    // Process remaining bytes
    while i < len {
        if data[i] < min || data[i] > max {
            return false;
        }
        i += 1;
    }
    
    true
}

/// Checksum calculation using SIMD
pub fn calculate_checksum_simd(data: &[u8]) -> u32 {
    let mut checksum = 0u32;
    let len = data.len();
    let mut i = 0;
    
    // Process 16 bytes at a time using SIMD when possible
    #[cfg(target_feature = "avx2")]
    if is_x86_feature_detected!("avx2") && len >= 16 {
        use std::arch::x86_64::*;
        
        let mut vec_sum = _mm256_setzero_si256();
        
        while i + 16 <= len {
            let chunk = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);
            vec_sum = _mm256_add_epi32(vec_sum, chunk);
            i += 16;
        }
        
        // Horizontal sum
        let sum1 = _mm256_extracti128_si256(vec_sum, 1);
        let sum0 = _mm256_castsi256_si128(vec_sum);
        let sum = _mm_add_epi32(sum0, sum1);
        
        let mut temp = [0u32; 4];
        _mm_storeu_si128(temp.as_mut_ptr() as *mut __m128i, sum);
        checksum = temp[0].wrapping_add(temp[1])
            .wrapping_add(temp[2])
            .wrapping_add(temp[3]);
    }
    
    // Process remaining bytes
    while i < len {
        checksum = checksum.wrapping_add(data[i] as u32);
        i += 1;
    }
    
    checksum
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simd_detection() {
        let features = detect_simd_features();
        println!("SIMD Features: {:?}", features);
        // Just verify it doesn't panic
    }
    
    #[test]
    fn test_write_u32_le_scalar() {
        let mut output = [0u8; 16];
        let values = [0x01020304, 0x05060708];
        
        write_u32_le_scalar(&mut output, &values);
        
        assert_eq!(output[0], 0x04);
        assert_eq!(output[1], 0x03);
        assert_eq!(output[2], 0x02);
        assert_eq!(output[3], 0x01);
        assert_eq!(output[4], 0x08);
        assert_eq!(output[5], 0x07);
        assert_eq!(output[6], 0x06);
        assert_eq!(output[7], 0x05);
    }
    
    #[test]
    fn test_segment_table_writing() {
        let mut output = [0u8; 32];
        let segment_counts = [3];
        let segment_sizes = [100, 200, 300];
        
        let written = write_segment_table_simd(&mut output, &segment_counts, &segment_sizes);
        
        assert!(written > 0);
        assert_eq!(output[0], 2); // segment_count - 1
        assert_eq!(output[4], 100); // first segment size
    }
    
    #[test]
    fn test_checksum_calculation() {
        let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let checksum = calculate_checksum_simd(&data);
        
        // Simple checksum: sum of all bytes
        let expected: u32 = data.iter().map(|&x| x as u32).sum();
        assert_eq!(checksum, expected);
    }
}