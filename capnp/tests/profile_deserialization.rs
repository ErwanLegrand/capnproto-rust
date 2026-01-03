// Profiling test for deserialization performance
use std::time::Instant;
use criterion::{criterion_group, criterion_main, Criterion};

fn create_test_message() -> Vec<u8> {
    use capnp::message::Builder;
    
    let mut message = Builder::new_default();
    message.set_root("This is a test message for profiling deserialization performance").unwrap();
    
    // Serialize the message
    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, &message).unwrap();
    
    buffer
}

fn benchmark_deserialization(c: &mut Criterion) {
    let serialized = create_test_message();
    
    c.bench_function("deserialize_text_message", |b| {
        b.iter(|| {
            let mut slice = &serialized[..];
            let _reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
        });
    });
}

#[test]
fn simple_deserialization_test() {
    println!("Running simple deserialization profiling...");
    
    let serialized = create_test_message();
    let iterations = 10000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let mut slice = &serialized[..];
        let _reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
    }
    let elapsed = start.elapsed();
    
    println!("Small message deserialization time for {} iterations: {:?}", iterations, elapsed);
    println!("Average time per small message: {:?}", elapsed / iterations as u32);
}

fn create_multi_segment_message() -> Vec<u8> {
    use capnp::message::{Builder, HeapAllocator};
    
    // Create a message with multiple segments by using a small first segment
    let mut allocator = HeapAllocator::new().first_segment_words(8); // Small first segment
    let mut message = Builder::new(&mut allocator);
    
    // This should create multiple segments
    let large_text = "A".repeat(1000);
    message.set_root(&large_text).unwrap();
    
    // Serialize the message
    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, &message).unwrap();
    
    buffer
}

#[test]
fn multi_segment_deserialization_test() {
    println!("Running multi-segment message deserialization profiling...");
    
    let serialized = create_multi_segment_message();
    println!("Serialized message size: {} bytes", serialized.len());
    
    let iterations = 1000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let mut slice = &serialized[..];
        let _reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
    }
    let elapsed = start.elapsed();
    
    println!("Multi-segment message deserialization time for {} iterations: {:?}", iterations, elapsed);
    println!("Average time per multi-segment message: {:?}", elapsed / iterations as u32);
}

fn create_four_segment_message() -> Vec<u8> {
    use capnp::message::{Builder, HeapAllocator};
    
    // Create a message with exactly 4 segments to trigger SIMD path
    let mut allocator = HeapAllocator::new().first_segment_words(8); // Small first segment
    let mut message = Builder::new(&mut allocator);
    
    // Create data that will span multiple segments
    let large_text = "A".repeat(2000); // Should create multiple segments
    message.set_root(&large_text).unwrap();
    
    // Serialize the message
    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, &message).unwrap();
    
    buffer
}

#[test]
fn four_segment_deserialization_test() {
    println!("Running 4+ segment message deserialization profiling...");
    
    let serialized = create_four_segment_message();
    println!("Serialized message size: {} bytes", serialized.len());
    
    let iterations = 1000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let mut slice = &serialized[..];
        let _reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
    }
    let elapsed = start.elapsed();
    
    println!("4+ segment message deserialization time for {} iterations: {:?}", iterations, elapsed);
    println!("Average time per 4+ segment message: {:?}", elapsed / iterations as u32);
}

criterion_group!(benches, benchmark_deserialization);
criterion_main!(benches);