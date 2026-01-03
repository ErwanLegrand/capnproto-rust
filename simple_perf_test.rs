// Simple performance test to establish baseline
use std::time::Instant;
use capnp::message::Builder;
use capnp::message::HeapAllocator;

fn main() {
    println!("Running simple performance baseline test...");
    
    // Test 1: Basic message building
    let start = Instant::now();
    for _ in 0..1000 {
        let mut message = Builder::new(HeapAllocator::new());
        let mut root = message.init_root::<capnp::data::Builder>();
        root.set(&[1, 2, 3, 4, 5]);
    }
    let duration = start.elapsed();
    println!("Basic message building (1000 iterations): {:?}", duration);
    
    // Test 2: Serialization
    let mut message = Builder::new(HeapAllocator::new());
    let mut root = message.init_root::<capnp::data::Builder>();
    root.set(&[1, 2, 3, 4, 5]);
    
    let start = Instant::now();
    for _ in 0..1000 {
        let mut buffer = Vec::new();
        capnp::serialize::write_message(&mut buffer, &message).unwrap();
    }
    let duration = start.elapsed();
    println!("Serialization (1000 iterations): {:?}", duration);
    
    // Test 3: Deserialization
    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, &message).unwrap();
    
    let start = Instant::now();
    for _ in 0..1000 {
        let mut slice = &buffer[..];
        let _reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();
    }
    let duration = start.elapsed();
    println!("Deserialization (1000 iterations): {:?}", duration);
    
    println!("Baseline test complete.");
}