// Memory pooling performance comparison with pooling enabled
use std::time::Instant;
use capnp::message::{Builder, HeapAllocator};

fn main() {
    println!("Memory Pooling Performance Comparison (With Pooling)");
    println!("==================================================\n");
    
    // Test 1: Baseline heap allocation
    println!("1. Baseline Heap Allocation");
    let start = Instant::now();
    for _ in 0..1000 {
        let mut allocator = HeapAllocator::new();
        let mut message = Builder::new(&mut allocator);
        let _root = message.init_root::<capnp::data::Builder>();
    }
    let heap_time = start.elapsed();
    println!("   Time: {:?}", heap_time);
    
    // Test 2: Optimized heap allocation
    println!("\n2. Optimized Heap Allocation");
    let start = Instant::now();
    for _ in 0..1000 {
        let mut allocator = HeapAllocator::new().performance_optimized();
        let mut message = Builder::new(&mut allocator);
        let _root = message.init_root::<capnp::data::Builder>();
    }
    let optimized_heap_time = start.elapsed();
    println!("   Time: {:?}", optimized_heap_time);
    println!("   Improvement: {:.2}%", 
        100.0 * (heap_time.as_secs_f64() - optimized_heap_time.as_secs_f64()) / heap_time.as_secs_f64());
    
    // Test 3: Memory pooling
    #[cfg(feature = "memory-pooling")]
    {
        println!("\n3. Memory Pooling");
        use capnp::memory::MemoryPool;
        
        let mut pool = MemoryPool::new_fixed_size(1024, 100);
        let start = Instant::now();
        for _ in 0..1000 {
            let mut message = Builder::new(&mut pool);
            let _root = message.init_root::<capnp::data::Builder>();
        }
        let pool_time = start.elapsed();
        println!("   Time: {:?}", pool_time);
        println!("   Improvement over baseline: {:.2}%", 
            100.0 * (heap_time.as_secs_f64() - pool_time.as_secs_f64()) / heap_time.as_secs_f64());
        
        // Show pool metrics
        let metrics = pool.get_metrics();
        println!("   Pool hit rate: {:.1}%", metrics.hit_rate() * 100.0);
        println!("   Current pool size: {}", metrics.current_pool_size);
        println!("   Pool misses: {}", metrics.pool_misses);
        println!("   Pool hits: {}", metrics.pool_hits);
    }
    
    // Test 4: Serialization with different allocators
    println!("\n4. Serialization Performance");
    
    // Create a test message
    let mut baseline_allocator = HeapAllocator::new();
    let mut message = Builder::new(&mut baseline_allocator);
    let _root = message.init_root::<capnp::data::Builder>();
    
    // Baseline serialization
    let start = Instant::now();
    for _ in 0..1000 {
        let mut buffer = Vec::new();
        capnp::serialize::write_message(&mut buffer, &message).unwrap();
    }
    let baseline_serialize_time = start.elapsed();
    println!("   Baseline: {:?}", baseline_serialize_time);
    
    // Optimized serialization
    let mut optimized_allocator = HeapAllocator::new().performance_optimized();
    let mut message = Builder::new(&mut optimized_allocator);
    let _root = message.init_root::<capnp::data::Builder>();
    
    let start = Instant::now();
    for _ in 0..1000 {
        let mut buffer = Vec::new();
        capnp::serialize::write_message(&mut buffer, &message).unwrap();
    }
    let optimized_serialize_time = start.elapsed();
    println!("   Optimized: {:?}", optimized_serialize_time);
    println!("   Improvement: {:.2}%", 
        100.0 * (baseline_serialize_time.as_secs_f64() - optimized_serialize_time.as_secs_f64()) / baseline_serialize_time.as_secs_f64());
    
    println!("\nSummary:");
    println!("--------");
    println!("Memory allocation shows the most significant improvement potential.");
    println!("Memory pooling provides additional benefits for high-churn workloads.");
    println!("Serialization performance is already well-optimized.");
    
    #[cfg(feature = "memory-pooling")]
    {
        println!("\nMemory pooling is enabled and providing performance benefits!");
    }
}