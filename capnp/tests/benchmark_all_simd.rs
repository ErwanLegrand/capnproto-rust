// Comprehensive benchmark for all SIMD optimizations
use std::time::Instant;
use capnp::message::Builder;
use capnp::primitive_list;
use capnp::serialize;

fn create_test_message() -> Vec<u8> {
    let mut message = Builder::new_default();
    message.set_root("This is a test message for comprehensive SIMD benchmarking").unwrap();
    
    let mut buffer = Vec::new();
    serialize::write_message(&mut buffer, &message).unwrap();
    buffer
}

fn create_multi_segment_message() -> Vec<u8> {
    // Create a message with 4 segments to trigger SIMD deserialization
    let mut buffer = Vec::new();
    
    // 4 segments: segment_count-1 = 3
    buffer.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]);
    // First segment size = 10 words
    buffer.extend_from_slice(&[0x0A, 0x00, 0x00, 0x00]);
    // Remaining segment sizes
    buffer.extend_from_slice(&[0x14, 0x00, 0x00, 0x00]); // 20 words
    buffer.extend_from_slice(&[0x1E, 0x00, 0x00, 0x00]); // 30 words
    buffer.extend_from_slice(&[0x28, 0x00, 0x00, 0x00]); // 40 words
    // Padding to 16 bytes
    buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    // Segment data (100 words = 800 bytes)
    buffer.extend_from_slice(&vec![0u8; 800]);
    
    buffer
}

fn benchmark_deserialization(iterations: usize) -> std::time::Duration {
    let buffer = create_multi_segment_message();
    
    let start = Instant::now();
    for _ in 0..iterations {
        let mut slice = &buffer[..];
        let _reader = serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
    }
    let elapsed = start.elapsed();
    
    println!("Deserialization benchmark:");
    println!("  {} iterations of 4-segment messages", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Average time per message: {:?}", elapsed / iterations as u32);
    
    elapsed
}

fn benchmark_primitive_list_operations(iterations: usize) -> std::time::Duration {
    let mut message = Builder::new_default();
    let mut list = message.initn_root::<primitive_list::Builder<'_, u32>>(1000);
    
    // Initialize with some data
    for i in 0..1000 {
        list.set(i, i as u32);
    }
    
    let source: Vec<u32> = (1000..2000).collect();
    
    let start = Instant::now();
    for _ in 0..iterations {
        #[cfg(feature = "simd")]
        {
            list.copy_from_slice_simd(&source);
        }
        
        #[cfg(not(feature = "simd"))]
        {
            for (i, &value) in source.iter().enumerate() {
                if i < 1000 {
                    list.set(i as u32, value);
                }
            }
        }
    }
    let elapsed = start.elapsed();
    
    println!("Primitive list operations benchmark:");
    println!("  {} iterations of 1000-element copies", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Average time per operation: {:?}", elapsed / iterations as u32);
    
    elapsed
}

fn benchmark_data_validation(iterations: usize) -> std::time::Duration {
    use capnp::simd::{validate_no_null_bytes_simd, validate_range_simd};
    
    // Create test data
    let test_data = b"Hello, World! This is a test string for validation benchmarking.";
    
    let start = Instant::now();
    for _ in 0..iterations {
        // Test both validation functions
        let _ = validate_no_null_bytes_simd(test_data);
        let _ = validate_range_simd(test_data, 32, 126);
    }
    let elapsed = start.elapsed();
    
    println!("Data validation benchmark:");
    println!("  {} iterations of validation checks", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Average time per validation: {:?}", elapsed / (iterations as u32 * 2));
    
    elapsed
}

#[test]
fn comprehensive_simd_benchmark() {
    println!("=== Comprehensive SIMD Optimizations Benchmark ===");
    
    const ITERATIONS: usize = 1000;
    
    // Benchmark deserialization
    let deserialization_time = benchmark_deserialization(ITERATIONS);
    
    // Benchmark primitive list operations
    let primitive_list_time = benchmark_primitive_list_operations(ITERATIONS);
    
    // Benchmark data validation
    let validation_time = benchmark_data_validation(ITERATIONS);
    
    let total_time = deserialization_time + primitive_list_time + validation_time;
    
    println!("\n=== Summary ===");
    println!("Total benchmark time: {:?}", total_time);
    println!("Deserialization: {:?} ({:.1}%)", deserialization_time, deserialization_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("Primitive lists: {:?} ({:.1}%)", primitive_list_time, primitive_list_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("Data validation: {:?} ({:.1}%)", validation_time, validation_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    
    #[cfg(feature = "simd")]
    println!("\nSIMD optimizations are ENABLED");
    
    #[cfg(not(feature = "simd"))]
    println!("\nSIMD optimizations are DISABLED");
}