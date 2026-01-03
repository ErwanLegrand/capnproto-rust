# Cap'n Proto Rust Optimization Guide

## Table of Contents

1. [Introduction](#introduction)
2. [General Optimization Principles](#general-optimization-principles)
3. [Memory Management](#memory-management)
4. [Serialization Performance](#serialization-performance)
5. [Primitive Access Patterns](#primitive-access-patterns)
6. [Batch Processing](#batch-processing)
7. [Async/Await Patterns](#asyncawait-patterns)
8. [Advanced Techniques](#advanced-techniques)
9. [Benchmarking and Profiling](#benchmarking-and-profiling)
10. [Common Pitfalls](#common-pitfalls)
11. [API Reference](#api-reference)

## Introduction

This guide provides best practices and optimization techniques for getting the most performance out of capnproto-rust. The library is already highly optimized, but understanding these patterns will help you write efficient code and choose the right configurations for your use case.

## General Optimization Principles

### 1. Zero-Copy Philosophy

Cap'n Proto is designed around zero-copy serialization. Embrace this philosophy:

```rust
// ✅ GOOD: Direct access to serialized data
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
let data = reader.get_root::<capnp::data::Reader>()?;
// data is a direct view into the serialized buffer

// ❌ BAD: Unnecessary copying
let data = reader.get_root::<capnp::data::Reader>()?;
let copied_data = data.to_vec(); // Avoid this unless absolutely necessary
```

### 2. Reuse Allocators

Reuse allocators to reduce allocation overhead:

```rust
// ✅ GOOD: Reuse allocator
let mut allocator = HeapAllocator::new().performance_optimized();
for _ in 0..1000 {
    let mut message = Builder::new(&mut allocator);
    // Build message...
}

// ❌ BAD: Create new allocator each time
for _ in 0..1000 {
    let mut message = Builder::new(HeapAllocator::new());
    // Build message...
}
```

### 3. Choose the Right Allocation Strategy

```rust
// For general purpose use
let allocator = HeapAllocator::new(); // Default strategy

// For performance-critical applications
let allocator = HeapAllocator::new().performance_optimized();

// For testing or specific workloads
let allocator = HeapAllocator::new().allocation_strategy(AllocationStrategy::FixedSize);
```

## Memory Management

### Allocator Configuration

#### HeapAllocator Options

| Option | Description | Best For |
|--------|-------------|----------|
| `GrowHeuristically` | Exponential growth (default) | General purpose |
| `PerformanceOptimized` | Aggressive growth | High-performance apps |
| `FixedSize` | Fixed segment sizes | Testing, predictable workloads |

#### Configuration Examples

```rust
// Default configuration (good for most cases)
let allocator = HeapAllocator::new();

// Performance-optimized for high-throughput applications
let allocator = HeapAllocator::new()
    .performance_optimized()
    .first_segment_words(2048);

// Fixed size for testing or embedded systems
let allocator = HeapAllocator::new()
    .allocation_strategy(AllocationStrategy::FixedSize)
    .first_segment_words(1024);

// Custom configuration for specific workloads
let allocator = HeapAllocator::new()
    .allocation_strategy(AllocationStrategy::GrowHeuristically)
    .first_segment_words(512)
    .max_segment_words(8192);
```

### Memory Pooling (Advanced)

For applications with high allocation churn, consider memory pooling:

```rust
// Create a memory pool
let mut pool = MemoryPool::new_default();

// Use the pool with your builder
let mut builder = Builder::new(&mut pool);

// The pool will automatically reuse segments
for _ in 0..1000 {
    let mut message = Builder::new(&mut pool);
    // Build and process messages...
}
```

### Segment Size Optimization

Choose appropriate segment sizes based on your message patterns:

```rust
// Small messages (few KB)
let allocator = HeapAllocator::new().first_segment_words(256);

// Medium messages (tens of KB)
let allocator = HeapAllocator::new().first_segment_words(1024);

// Large messages (hundreds of KB+)
let allocator = HeapAllocator::new().first_segment_words(4096);
```

## Serialization Performance

### Serialization Best Practices

#### 1. Batch Serialization

```rust
// ✅ GOOD: Batch multiple messages
let mut buffer = Vec::new();
for message in messages {
    capnp::serialize::write_message(&mut buffer, &message)?;
}

// ❌ BAD: Individual serialization with separate buffers
for message in messages {
    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, &message)?;
    // Process buffer...
}
```

#### 2. Reuse Buffers

```rust
// ✅ GOOD: Reuse serialization buffer
let mut buffer = Vec::with_capacity(65536); // Pre-allocate
for _ in 0..1000 {
    buffer.clear();
    let mut message = Builder::new(HeapAllocator::new());
    // Build message...
    capnp::serialize::write_message(&mut buffer, &message)?;
}

// ❌ BAD: Create new buffer each time
for _ in 0..1000 {
    let mut buffer = Vec::new();
    let mut message = Builder::new(HeapAllocator::new());
    // Build message...
    capnp::serialize::write_message(&mut buffer, &message)?;
}
```

#### 3. Packed vs Unpacked Serialization

```rust
// Packed serialization (smaller, slightly slower)
use capnp::serialize_packed;
let mut buffer = Vec::new();
serialize_packed::write_message(&mut buffer, &message)?;

// Unpacked serialization (faster, larger)
use capnp::serialize;
let mut buffer = Vec::new();
serialize::write_message(&mut buffer, &message)?;
```

**Choose packed when:** Network bandwidth is limited
**Choose unpacked when:** CPU is the bottleneck

### Deserialization Optimization

#### 1. Direct Access Patterns

```rust
// ✅ GOOD: Direct access to deserialized data
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
let data = reader.get_root::<capnp::data::Reader>()?;
// Work with data directly

// ❌ BAD: Unnecessary copying
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
let data = reader.get_root::<capnp::data::Reader>()?;
let copied = data.to_vec(); // Avoid this
```

#### 2. Reader Options

```rust
// Default options (good for most cases)
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;

// Custom options for specific needs
let options = capnp::message::ReaderOptions {
    traversal_limit_in_words: Some(1000000), // Increase for large messages
    nesting_limit: 64, // Adjust based on your schema depth
};
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, options)?;
```

## Primitive Access Patterns

### Efficient Data Access

```rust
// ✅ GOOD: Direct slice access for primitive lists
let list = reader.get_list::<u32>()?;
if let Some(slice) = list.as_slice() {
    // Work with slice directly (zero-copy)
    for &value in slice {
        // Process value...
    }
}

// ❌ BAD: Individual element access
let list = reader.get_list::<u32>()?;
for i in 0..list.len() {
    let value = list.get(i); // Multiple bounds checks
    // Process value...
}
```

### Primitive List Optimization

```rust
// ✅ GOOD: Use try_get for bounds-checked access
let list = reader.get_list::<u32>()?;
for i in 0..list.len() {
    if let Some(value) = list.try_get(i) {
        // Process value...
    }
}

// ✅ GOOD: Use iter() for sequential access
let list = reader.get_list::<u32>()?;
for value in list.iter() {
    // Process value...
}

// ❌ BAD: Manual bounds checking
let list = reader.get_list::<u32>()?;
for i in 0..list.len() {
    if i < list.len() { // Redundant check
        let value = list.get(i);
        // Process value...
    }
}
```

## Batch Processing

### Batch Processing Patterns

#### 1. Sequential Processing

```rust
// ✅ GOOD: Sequential batch processing
let messages: Vec<Vec<u8>> = get_messages();
let mut results = Vec::with_capacity(messages.len());

for message in messages {
    let mut slice = &message[..];
    let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
    let result = process_message(&reader)?;
    results.push(result);
}
```

#### 2. Parallel Processing

```rust
// ✅ GOOD: Parallel batch processing
use rayon::prelude::*;

let messages: Vec<Vec<u8>> = get_messages();
let results: Vec<Result<ProcessedResult>> = messages.par_iter()
    .map(|message| {
        let mut slice = &message[..];
        let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
        process_message(&reader)
    })
    .collect();
```

#### 3. Pipelined Processing

```rust
// ✅ GOOD: Pipeline processing
use std::sync::mpsc;
use rayon::prelude::*;

let (tx, rx) = mpsc::channel();

// Stage 1: Deserialization
messages.par_iter().for_each(|message| {
    let mut slice = &message[..];
    let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default());
    tx.send(reader).unwrap();
});

// Stage 2: Processing
let processed: Vec<_> = rx.par_iter()
    .map(|reader| process_message(&reader))
    .collect();
```

### Batch Configuration

```rust
// Small batches (low latency)
let batch_size = 16;

// Medium batches (balanced)
let batch_size = 64;

// Large batches (high throughput)
let batch_size = 256;
```

## Async/Await Patterns

### Async Serialization

```rust
// ✅ GOOD: Async serialization with tokio
use tokio::io::AsyncWriteExt;

async fn serialize_async<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    message: &capnp::message::Builder<impl capnp::message::Allocator>
) -> Result<()> {
    let mut buffer = Vec::new();
    capnp::serialize::write_message(&mut buffer, message)?;
    writer.write_all(&buffer).await?;
    Ok(())
}
```

### Async Deserialization

```rust
// ✅ GOOD: Async deserialization with tokio
use tokio::io::AsyncReadExt;

async fn deserialize_async<R: AsyncReadExt + Unpin>(
    reader: &mut R
) -> Result<capnp::message::Reader<capnp::serialize::OwnedSegments>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).await?;
    capnp::serialize::read_message(&mut &buffer[..], Default::default())
}
```

### Async Batch Processing

```rust
// ✅ GOOD: Async batch processing
use futures::stream::StreamExt;

async fn process_batch_async(
    messages: impl Stream<Item = Vec<u8>> + Unpin
) -> Result<Vec<ProcessedResult>> {
    let mut results = Vec::new();
    
    let mut stream = messages;
    while let Some(message) = stream.next().await {
        let mut slice = &message[..];
        let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
        let result = process_message(&reader)?;
        results.push(result);
    }
    
    Ok(results)
}
```

## Advanced Techniques

### Custom Allocators

```rust
// Implement custom allocator for specific needs
pub struct ArenaAllocator {
    arena: bumpalo::Bump,
}

unsafe impl capnp::message::Allocator for ArenaAllocator {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
        let size = minimum_size as usize * 8;
        let ptr = self.arena.alloc_layout(
            Layout::from_size_align(size, 8).unwrap()
        );
        (ptr.as_ptr() as *mut u8, minimum_size)
    }
    
    unsafe fn deallocate_segment(&mut self, _ptr: *mut u8, _word_size: u32, _words_used: u32) {
        // Arena allocator doesn't deallocate until reset
    }
}

// Use custom allocator
let arena = bumpalo::Bump::new();
let mut allocator = ArenaAllocator { arena };
let mut message = Builder::new(&mut allocator);
```

### Memory Mapping

```rust
// ✅ GOOD: Memory-mapped file access
use memmap2::Mmap;

let file = std::fs::File::open("data.capnp")?;
let mmap = unsafe { Mmap::map(&file)? };
let mut slice = &mmap[..];

let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
// Work with memory-mapped data
```

### Zero-Copy Networking

```rust
// ✅ GOOD: Zero-copy networking with bytes
use bytes::Bytes;

let bytes: Bytes = get_network_data();
let mut slice = &bytes[..];

let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
// Process data without copying
```

## Benchmarking and Profiling

### Running Benchmarks

```bash
# Run all benchmarks
cargo run --release --bin capnp-bench

# Run specific benchmark
cargo run --release --bin capnp-bench run serialization_large

# Run with custom configuration
cargo run --release --bin capnp-bench run --iterations 1000 --warmup 100

# Compare with baseline
cargo run --release --bin capnp-bench compare baseline.json current.json
```

### Profiling Techniques

```bash
# CPU profiling with perf
perf record -- cargo run --release --bin your_app
perf report

# Memory profiling with heaptrack
heaptrack cargo run --release --bin your_app

# Flamegraph generation
cargo flamegraph --bin your_app
```

### Benchmark Analysis

```rust
// Analyze benchmark results
let results = run_benchmarks();
let stats = results.calculate_statistics();

println!("Benchmark: {}", stats.name);
println!("Mean: {:.3} ms", stats.mean * 1000.0);
println!("Median: {:.3} ms", stats.median * 1000.0);
println!("Std Dev: {:.3} ms", stats.std_dev * 1000.0);
println!("Throughput: {:.0} ops/s", stats.throughput);
```

## Common Pitfalls

### 1. Unnecessary Copies

```rust
// ❌ BAD: Unnecessary data copying
let data = reader.get_data()?;
let copied = data.to_vec(); // Avoid this

// ✅ GOOD: Direct access
let data = reader.get_data()?;
// Work with data directly
```

### 2. Inefficient Allocation

```rust
// ❌ BAD: Creating new allocators in loops
for _ in 0..1000 {
    let mut message = Builder::new(HeapAllocator::new());
}

// ✅ GOOD: Reusing allocators
let mut allocator = HeapAllocator::new();
for _ in 0..1000 {
    let mut message = Builder::new(&mut allocator);
}
```

### 3. Poor Segment Sizing

```rust
// ❌ BAD: Too small segments for large messages
let allocator = HeapAllocator::new().first_segment_words(64);
// This will cause many segment allocations for large messages

// ✅ GOOD: Appropriate segment size
let allocator = HeapAllocator::new().first_segment_words(2048);
```

### 4. Ignoring Error Handling

```rust
// ❌ BAD: Ignoring potential errors
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default()).unwrap();

// ✅ GOOD: Proper error handling
let reader = capnp::serialize::read_message_from_flat_slice(&mut slice, Default::default())?;
```

### 5. Overusing Dynamic Dispatch

```rust
// ❌ BAD: Excessive dynamic dispatch
let dynamic = reader.get_as_dynamic()?;
// Dynamic access is slower than typed access

// ✅ GOOD: Typed access when possible
let typed = reader.get_root::<MyStruct::Reader>()?;
```

## API Reference

### Allocator Configuration

```rust
// HeapAllocator methods
HeapAllocator::new()
    .first_segment_words(1024)
    .allocation_strategy(AllocationStrategy::PerformanceOptimized)
    .max_segment_words(8192)
    .performance_optimized()
```

### Serialization Methods

```rust
// Unpacked serialization
capnp::serialize::write_message(writer, message)
capnp::serialize::read_message(reader, options)

// Packed serialization
capnp::serialize_packed::write_message(writer, message)
capnp::serialize_packed::read_message(reader, options)

// Flat slice operations
capnp::serialize::read_message_from_flat_slice(slice, options)
capnp::serialize::write_message_to_flat_slice(writer, message)
```

### Reader Options

```rust
capnp::message::ReaderOptions {
    traversal_limit_in_words: Some(1000000),
    nesting_limit: 64,
}
```

### Memory Pooling (when available)

```rust
MemoryPool::new_default()
MemoryPool::new_fixed_size(segment_size, capacity)
MemoryPool::new_variable_size(size_classes, capacity)
```

## Performance Checklist

### Before Optimization

- [ ] Profile your application to identify bottlenecks
- [ ] Understand your workload characteristics
- [ ] Establish performance baselines
- [ ] Set realistic performance goals

### During Optimization

- [ ] Apply zero-copy principles
- [ ] Reuse allocators and buffers
- [ ] Choose appropriate allocation strategies
- [ ] Use batch processing where possible
- [ ] Optimize primitive access patterns
- [ ] Consider async patterns for I/O-bound workloads

### After Optimization

- [ ] Verify correctness
- [ ] Measure performance improvements
- [ ] Check for regressions
- [ ] Document changes
- [ ] Update baselines

## Conclusion

This optimization guide provides comprehensive best practices for getting the most performance out of capnproto-rust. Remember that:

1. **Zero-copy is key**: Embrace Cap'n Proto's zero-copy design
2. **Reuse resources**: Allocators, buffers, and builders should be reused
3. **Choose wisely**: Select appropriate configurations for your workload
4. **Measure everything**: Always validate optimizations with benchmarks
5. **Profile first**: Identify real bottlenecks before optimizing

By following these principles and patterns, you can achieve optimal performance while maintaining the safety and correctness guarantees that capnproto-rust provides.

## Additional Resources

- [Cap'n Proto Specification](https://capnproto.org/encoding.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Benchmarking Guide](benchmarking_framework.md)
- [Memory Pooling Design](memory_pool_design.md)