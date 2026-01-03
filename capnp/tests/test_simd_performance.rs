// Test SIMD performance improvements
use std::time::Instant;

#[test]
fn test_simd_performance() {
    use capnp::message::{Builder, HeapAllocator};
    use capnp::serialize;

    const ITERATIONS: u32 = 1000;
    const MESSAGE_SIZE: usize = 4096; // Size in words (larger to see SIMD benefits)

    println!("Testing Cap'n Proto serialization with SIMD...");

    let start = Instant::now();
    
    for _ in 0..ITERATIONS {
        let mut message = Builder::new(HeapAllocator::new());
        let root = message.init_root::<capnp::data::Builder<'_>>();
        // The data is already initialized, we just need to serialize it
        
        let mut buffer = Vec::new();
        serialize::write_message(&mut buffer, &message).unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("SIMD Serialization time for {} iterations: {:?}", ITERATIONS, elapsed);
    println!("Average time per message: {:?}", elapsed / ITERATIONS);
}

#[test]
fn test_simd_functions() {
    use capnp::simd::{write_u32_le_simd, detect_simd_features};
    
    let features = detect_simd_features();
    println!("SIMD Features: {:?}", features);
    
    // Test the SIMD function
    let values = [0x01020304u32, 0x05060708, 0x090A0B0C, 0x0D0E0F10];
    let mut output = [0u8; 16];
    
    write_u32_le_simd(&mut output, &values);
    
    // Verify correctness
    assert_eq!(output[0], 0x04); // 0x01020304 little-endian
    assert_eq!(output[1], 0x03);
    assert_eq!(output[2], 0x02);
    assert_eq!(output[3], 0x01);
    
    assert_eq!(output[4], 0x08); // 0x05060708 little-endian
    assert_eq!(output[5], 0x07);
    assert_eq!(output[6], 0x06);
    assert_eq!(output[7], 0x05);
    
    println!("SIMD functions working correctly!");
}