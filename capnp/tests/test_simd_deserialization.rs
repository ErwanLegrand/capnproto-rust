// Test SIMD deserialization specifically
use capnp::message::{ReaderSegments};
use capnp::serialize;

fn create_manual_multi_segment_message() -> Vec<u8> {
    // Manually create a message with 4 segments to trigger SIMD path
    // Format: [segment_count-1][seg1_size][seg2_size][seg3_size][seg4_size][data...]
    
    // 4 segments: segment_count-1 = 3 (0x03 0x00 0x00 0x00)
    // Segment sizes: 10, 20, 30, 40 words each
    // Total words: 10 + 20 + 30 + 40 = 100 words = 800 bytes
    
    let mut buffer = Vec::new();
    
    // First 8 bytes: segment_count-1 (4 bytes) + first segment size (4 bytes)
    buffer.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]); // segment_count-1 = 3
    buffer.extend_from_slice(&[0x0A, 0x00, 0x00, 0x00]); // first segment size = 10 words
    
    // Remaining segment sizes (3 more segments)
    buffer.extend_from_slice(&[0x14, 0x00, 0x00, 0x00]); // 20 words
    buffer.extend_from_slice(&[0x1E, 0x00, 0x00, 0x00]); // 30 words
    buffer.extend_from_slice(&[0x28, 0x00, 0x00, 0x00]); // 40 words
    
    // Since we have 4 segments and (4 & !1) = 4, we need to pad to 4*4=16 bytes
    // But we already have 3 segment sizes (12 bytes), so we need 4 more bytes of padding
    buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // padding
    
    // Segment data (100 words = 800 bytes of zeros)
    buffer.extend_from_slice(&vec![0u8; 800]);
    
    buffer
}

fn benchmark_multi_segment_deserialization(iterations: usize) -> std::time::Duration {
    let buffer = create_manual_multi_segment_message();
    
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut slice = &buffer[..];
        let _reader = serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
    }
    let elapsed = start.elapsed();
    
    println!("Deserialized {} messages in {:?}", iterations, elapsed);
    println!("Average time per message: {:?}", elapsed / iterations as u32);
    
    elapsed
}

#[test]
fn test_simd_deserialization_manual() {
    // Create a message with exactly 4 segments manually
    // This should trigger the SIMD path (segment_count > 3)
    
    let buffer = create_manual_multi_segment_message();
    println!("Manual 4-segment message size: {} bytes", buffer.len());
    
    // Now try to deserialize it
    let mut slice = &buffer[..];
    let result = serialize::read_message_from_flat_slice(&mut slice, Default::default());
    
    match result {
        Ok(reader) => {
            println!("Successfully deserialized 4-segment message");
            
            // Check how many segments the message has
            let segment_count = reader.get_segments().len();
            println!("Message has {} segments", segment_count);
            
            if segment_count >= 4 {
                println!("SIMD path was triggered!");
            } else {
                println!("SIMD path was NOT triggered");
            }
        }
        Err(e) => {
            println!("Failed to deserialize: {:?}", e);
            panic!("Deserialization failed");
        }
    }
}

#[test]
fn benchmark_simd_deserialization() {
    println!("Benchmarking SIMD deserialization performance...");
    
    // Run with enough iterations to see meaningful results
    let iterations = 1000;
    let elapsed = benchmark_multi_segment_deserialization(iterations);
    
    println!("SIMD deserialization benchmark completed");
    println!("Total time for {} iterations: {:?}", iterations, elapsed);
    println!("Average time per 4-segment message: {:?}", elapsed / iterations as u32);
}

#[test]
fn test_read_u32_le_simd_function() {
    // Test our SIMD function directly
    use capnp::simd::read_u32_le_simd;
    
    // Create test data: 8 u32 values in little-endian
    let input = [
        0x01, 0x00, 0x00, 0x00, // 1
        0x02, 0x00, 0x00, 0x00, // 2
        0x03, 0x00, 0x00, 0x00, // 3
        0x04, 0x00, 0x00, 0x00, // 4
        0x05, 0x00, 0x00, 0x00, // 5
        0x06, 0x00, 0x00, 0x00, // 6
        0x07, 0x00, 0x00, 0x00, // 7
        0x08, 0x00, 0x00, 0x00, // 8
    ];
    
    let mut output = [0u32; 8];
    
    read_u32_le_simd(&input, &mut output);
    
    // Verify the output
    assert_eq!(output[0], 1);
    assert_eq!(output[1], 2);
    assert_eq!(output[2], 3);
    assert_eq!(output[3], 4);
    assert_eq!(output[4], 5);
    assert_eq!(output[5], 6);
    assert_eq!(output[6], 7);
    assert_eq!(output[7], 8);
    
    println!("SIMD read_u32_le_simd function works correctly");
}

#[test]
fn test_data_validation_simd() {
    // Test our SIMD data validation functions
    use capnp::simd::{validate_no_null_bytes_simd, validate_range_simd};
    
    // Test data without null bytes
    let valid_data = b"Hello, World! This is a test string without null bytes.";
    assert!(validate_no_null_bytes_simd(valid_data));
    
    // Test data with null bytes
    let invalid_data = b"Hello\x00World";
    assert!(!validate_no_null_bytes_simd(invalid_data));
    
    // Test empty data
    let empty_data = b"";
    assert!(validate_no_null_bytes_simd(empty_data));
    
    // Test range validation - valid ASCII range
    let ascii_data = b"Hello123";
    assert!(validate_range_simd(ascii_data, 32, 126));
    
    // Test range validation - contains out-of-range bytes
    let mixed_data = b"Hello\x01\xFF";
    assert!(!validate_range_simd(mixed_data, 32, 126));
    
    // Test range validation - all valid
    let valid_range_data = [65u8; 32]; // All 'A' characters
    assert!(validate_range_simd(&valid_range_data, 65, 65));
    
    println!("SIMD data validation functions work correctly");
}