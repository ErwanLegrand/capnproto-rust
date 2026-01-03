# Memory Pooling Design for Cap'n Proto Rust

## Overview
This document outlines the design for memory pooling optimizations to reduce allocation overhead in capnproto-rust.

## Current Architecture Analysis

### Current Allocation Pattern
```rust
// Current flow:
Message Builder → HeapAllocator → System Allocator
  ↑
Segment Allocation
```

### Performance Bottlenecks
1. **Frequent allocations**: Each segment requires separate allocation
2. **System allocator overhead**: malloc/free calls are expensive
3. **Fragmentation**: Mixed segment sizes cause memory waste
4. **No reuse**: Segments are freed but not reused

## Proposed Pooling Architecture

### High-Level Design
```rust
// New flow:
Message Builder → MemoryPool → (Reused Segments or HeapAllocator)
  ↑
Segment Pooling
```

### Core Components

#### 1. Segment Pool
```rust
pub struct SegmentPool {
    /// Pre-allocated segments ready for reuse
    free_segments: Vec<PooledSegment>,
    
    /// Statistics for monitoring
    metrics: PoolMetrics,
    
    /// Configuration
    config: PoolConfig,
}

struct PooledSegment {
    ptr: *mut u8,
    size: u32,
    generation: u64,
    last_used: Instant,
}
```

#### 2. Pooling Strategies

**Fixed-Size Pooling:**
```rust
struct FixedSizePool {
    segment_size: u32,
    free_list: Vec<*mut u8>,
    capacity: usize,
    hit_rate: f32,
}
```

**Variable-Size Pooling:**
```rust
const SIZE_CLASSES: usize = 8;
const SIZE_CLASS_SIZES: [u32; SIZE_CLASSES] = [64, 128, 256, 512, 1024, 2048, 4096, 8192];

struct VariableSizePool {
    size_classes: [Vec<*mut u8>; SIZE_CLASSES],
    allocation_stats: [usize; SIZE_CLASSES],
}
```

#### 3. Memory Pool Manager
```rust
pub struct MemoryPool {
    /// Primary pooling strategy
    pooling_strategy: PoolingStrategy,
    
    /// Fallback for large allocations
    fallback_allocator: HeapAllocator,
    
    /// Runtime metrics
    metrics: PoolMetrics,
    
    /// Configuration
    config: PoolConfig,
}

pub enum PoolingStrategy {
    FixedSize(FixedSizePool),
    VariableSize(VariableSizePool),
    Hybrid(HybridPool),
    Adaptive(AdaptivePool),
}
```

## Pooling Algorithms

### 1. Allocation Algorithm
```rust
fn allocate_segment(&mut self, size: u32) -> (*mut u8, u32) {
    // 1. Try to find exact match in pool
    if let Some(segment) = self.find_in_pool(size) {
        self.metrics.pool_hits += 1;
        return segment;
    }
    
    // 2. Try to find larger segment that can be used
    if let Some(segment) = self.find_larger_in_pool(size) {
        self.metrics.pool_hits += 1;
        return segment;
    }
    
    // 3. Allocate new segment
    self.metrics.pool_misses += 1;
    let segment = self.fallback_allocator.allocate_segment(size);
    
    // 4. Consider adding to pool for future reuse
    if self.should_cache(size) {
        self.add_to_pool(segment.0, segment.1);
    }
    
    segment
}
```

### 2. Reuse Algorithm
```rust
fn deallocate_segment(&mut self, ptr: *mut u8, size: u32) {
    // 1. Validate segment
    if !self.is_valid_segment(ptr, size) {
        unsafe { alloc::alloc::dealloc(ptr, Layout::from_size_align(size as usize * 8, 8).unwrap()) };
        return;
    }
    
    // 2. Add to appropriate pool
    match self.pooling_strategy {
        PoolingStrategy::FixedSize(ref mut pool) => {
            if size == pool.segment_size {
                pool.free_list.push(ptr);
            } else {
                // Too large for this pool, deallocate
                unsafe { alloc::alloc::dealloc(ptr, Layout::from_size_align(size as usize * 8, 8).unwrap()) };
            }
        }
        PoolingStrategy::VariableSize(ref mut pool) => {
            if let Some(class_idx) = self.find_size_class(size) {
                pool.size_classes[class_idx].push(ptr);
            } else {
                // Too large for any pool, deallocate
                unsafe { alloc::alloc::dealloc(ptr, Layout::from_size_align(size as usize * 8, 8).unwrap()) };
            }
        }
        // ... other strategies
    }
    
    // 3. Update metrics
    self.metrics.segments_reused += 1;
    self.metrics.current_pool_size += 1;
}
```

## Integration with Existing Code

### 1. Allocator Trait Implementation
```rust
unsafe impl Allocator for MemoryPool {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
        // Use pooling logic
        self.allocate_with_pooling(minimum_size)
    }
    
    unsafe fn deallocate_segment(&mut self, ptr: *mut u8, word_size: u32, words_used: u32) {
        // Use pooling logic
        self.deallocate_with_pooling(ptr, word_size, words_used);
        
        // Reset allocator state if needed
        if self.config.reset_on_deallocate {
            self.reset_allocation_strategy();
        }
    }
}
```

### 2. Builder Integration
```rust
// Modified Builder to support pooling
pub struct Builder<A = MemoryPool> {
    arena: BuilderArenaImpl<A>,
}

impl Builder<MemoryPool> {
    /// Creates a new Builder with memory pooling
    pub fn new_with_pooling(pool: MemoryPool) -> Self {
        Self {
            arena: BuilderArenaImpl::new(pool),
        }
    }
}
```

## Performance Optimization Techniques

### 1. Thread-Local Pooling
```rust
#[thread_local]
static THREAD_LOCAL_POOL: OnceCell<MemoryPool> = OnceCell::new();

fn get_thread_local_pool() -> &'static mut MemoryPool {
    THREAD_LOCAL_POOL.get_or_init(|| MemoryPool::new_default())
}
```

### 2. Adaptive Pooling
```rust
struct AdaptivePool {
    current_strategy: PoolingStrategy,
    performance_history: Vec<PoolPerformance>,
    adaptation_policy: AdaptationPolicy,
}

impl AdaptivePool {
    fn adapt_strategy(&mut self) {
        let current_performance = self.analyze_performance();
        
        match current_performance {
            Performance::HighFragmentation => {
                // Switch to variable-size pooling
                self.current_strategy = self.create_variable_size_pool();
            }
            Performance::LowHitRate => {
                // Adjust pool sizes
                self.adjust_pool_sizes();
            }
            Performance::HighContention => {
                // Switch to thread-local pooling
                self.enable_thread_local_mode();
            }
            _ => {
                // Keep current strategy
            }
        }
    }
}
```

### 3. Generational Pooling
```rust
struct GenerationalPool {
    current_generation: u64,
    segments: HashMap<u64, PooledSegment>,
    generation_stats: Vec<GenerationStats>,
}

impl GenerationalPool {
    fn cleanup_old_generations(&mut self) {
        let cutoff = self.current_generation.saturating_sub(MAX_GENERATIONS);
        
        self.segments.retain(|&gen, _| gen >= cutoff);
        
        // Update stats
        self.generation_stats.retain(|stats| stats.generation >= cutoff);
    }
}
```

## Metrics and Monitoring

### Pool Metrics Structure
```rust
pub struct PoolMetrics {
    /// Total allocations served by pool
    pub pool_hits: usize,
    
    /// Allocations that required new memory
    pub pool_misses: usize,
    
    /// Segments currently in pool
    pub current_pool_size: usize,
    
    /// Maximum pool size reached
    pub max_pool_size: usize,
    
    /// Total bytes managed by pool
    pub bytes_managed: usize,
    
    /// Bytes saved through reuse
    pub bytes_saved: usize,
    
    /// Allocation latency statistics
    pub allocation_latency: Histogram,
    
    /// Fragmentation metrics
    pub fragmentation_ratio: f32,
}
```

### Performance Monitoring
```rust
impl MemoryPool {
    pub fn get_metrics(&self) -> PoolMetrics {
        self.metrics.clone()
    }
    
    pub fn get_hit_rate(&self) -> f32 {
        let total = self.metrics.pool_hits + self.metrics.pool_misses;
        if total == 0 {
            0.0
        } else {
            self.metrics.pool_hits as f32 / total as f32
        }
    }
    
    pub fn get_memory_savings(&self) -> usize {
        // Calculate bytes saved through reuse
        self.metrics.bytes_saved
    }
}
```

## Configuration Options

### Pool Configuration
```rust
pub struct PoolConfig {
    /// Maximum number of segments to cache
    pub max_pool_size: usize,
    
    /// Segment size for fixed-size pooling
    pub fixed_segment_size: Option<u32>,
    
    /// Size classes for variable-size pooling
    pub size_classes: Option<Vec<u32>>,
    
    /// Maximum segment age before cleanup
    pub max_segment_age: Duration,
    
    /// Whether to reset allocator on deallocation
    pub reset_on_deallocate: bool,
    
    /// Adaptive pooling enabled
    pub adaptive: bool,
    
    /// Thread-local pooling enabled
    pub thread_local: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 1024,
            fixed_segment_size: Some(1024),
            size_classes: None,
            max_segment_age: Duration::from_secs(60),
            reset_on_deallocate: true,
            adaptive: false,
            thread_local: false,
        }
    }
}
```

## Error Handling and Safety

### Safety Considerations
```rust
impl MemoryPool {
    /// Safety: Ensures all pointers are properly aligned and valid
    unsafe fn validate_segment(&self, ptr: *mut u8, size: u32) -> bool {
        if ptr.is_null() {
            return false;
        }
        
        // Check alignment
        if (ptr as usize) % 8 != 0 {
            return false;
        }
        
        // Check size is reasonable
        if size == 0 || size > self.config.max_segment_size {
            return false;
        }
        
        true
    }
}
```

### Error Handling
```rust
#[derive(Debug)]
pub enum PoolError {
    PoolExhausted,
    InvalidSegment,
    SizeMismatch,
    AlignmentError,
    CapacityExceeded,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::PoolExhausted => write!(f, "Memory pool exhausted"),
            PoolError::InvalidSegment => write!(f, "Invalid segment pointer"),
            PoolError::SizeMismatch => write!(f, "Segment size mismatch"),
            PoolError::AlignmentError => write!(f, "Alignment error"),
            PoolError::CapacityExceeded => write!(f, "Pool capacity exceeded"),
        }
    }
}

impl std::error::Error for PoolError {}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_size_pool() {
        let mut pool = MemoryPool::new_fixed_size(1024, 100);
        
        // Test allocation
        let (ptr, size) = pool.allocate_segment(512);
        assert_eq!(size, 1024); // Should round up to pool size
        
        // Test deallocation
        unsafe { pool.deallocate_segment(ptr, size, 512) };
        
        // Test reuse
        let (ptr2, size2) = pool.allocate_segment(256);
        assert_eq!(size2, 1024);
        assert_eq!(pool.metrics.pool_hits, 1);
    }
    
    #[test]
    fn test_variable_size_pool() {
        let size_classes = vec![256, 512, 1024, 2048];
        let mut pool = MemoryPool::new_variable_size(size_classes, 50);
        
        // Test different size allocations
        let (ptr1, size1) = pool.allocate_segment(200); // Should use 256 class
        let (ptr2, size2) = pool.allocate_segment(600); // Should use 1024 class
        
        assert_eq!(size1, 256);
        assert_eq!(size2, 1024);
        
        // Test deallocation and reuse
        unsafe { 
            pool.deallocate_segment(ptr1, size1, 200);
            pool.deallocate_segment(ptr2, size2, 600);
        }
        
        let (ptr3, size3) = pool.allocate_segment(300); // Should reuse 256 class
        assert_eq!(size3, 256);
        assert_eq!(pool.metrics.pool_hits, 1);
    }
}
```

### Benchmark Tests
```rust
#[cfg(feature = "bench")]
mod benchmarks {
    use super::*;
    use test::Bencher;
    
    #[bench]
    fn bench_pool_allocation(b: &mut Bencher) {
        let mut pool = MemoryPool::new_fixed_size(1024, 1000);
        
        b.iter(|| {
            for _ in 0..100 {
                let (ptr, size) = pool.allocate_segment(512);
                unsafe { pool.deallocate_segment(ptr, size, 512) };
            }
        });
    }
    
    #[bench]
    fn bench_pool_vs_heap(b: &mut Bencher) {
        let mut pool = MemoryPool::new_fixed_size(1024, 1000);
        let mut heap_allocator = HeapAllocator::new();
        
        b.iter(|| {
            // Pool allocation
            for _ in 0..50 {
                let (ptr, size) = pool.allocate_segment(512);
                unsafe { pool.deallocate_segment(ptr, size, 512) };
            }
            
            // Heap allocation
            for _ in 0..50 {
                let (ptr, size) = heap_allocator.allocate_segment(512);
                unsafe { heap_allocator.deallocate_segment(ptr, size, 512) };
            }
        });
    }
}
```

## Integration Plan

### Step 1: Initial Implementation
```bash
# Create memory pool module
mkdir -p capnp/src/memory
touch capnp/src/memory/mod.rs
touch capnp/src/memory/pool.rs
```

### Step 2: Module Structure
```rust
// capnp/src/memory/mod.rs
pub mod pool;

pub use pool::{MemoryPool, PoolConfig, PoolMetrics, PoolError};
```

### Step 3: Feature Flag
```toml
# Cargo.toml
[features]
memory-pooling = []
```

### Step 4: Conditional Compilation
```rust
#[cfg(feature = "memory-pooling")]
pub mod memory;
```

## Migration Path

### Phase 1: Opt-in Usage
```rust
// Users can opt-in to pooling
let mut pool = MemoryPool::new_default();
let mut builder = Builder::new(&mut pool);
```

### Phase 2: Default Integration
```rust
// Eventually make pooling the default
let mut builder = Builder::new_with_pooling();
```

### Phase 3: Automatic Detection
```rust
// Smart detection of when to use pooling
let mut builder = Builder::new_auto(); // Uses pooling when beneficial
```

## Expected Performance Improvements

| Workload Type | Expected Improvement | Confidence Level |
|---------------|----------------------|------------------|
| Small messages | 15-25% | High |
| Mixed workloads | 20-35% | Medium |
| Batch processing | 25-40% | High |
| High churn | 30-50% | High |
| Large messages | 5-15% | Low |

## Risk Assessment

### Technical Risks
1. **Memory Leaks**: Improper pool management could cause leaks
2. **Fragmentation**: Poor pooling strategy could increase fragmentation
3. **Contention**: Thread-safe pooling could introduce contention
4. **Complexity**: Increased code complexity could reduce maintainability

### Mitigation Strategies
1. **Extensive Testing**: Comprehensive test suite
2. **Metrics Monitoring**: Real-time pool monitoring
3. **Fallback Mechanisms**: Graceful degradation to heap allocation
4. **Documentation**: Clear usage guidelines and best practices

## Future Enhancements

### 1. Cross-Thread Pooling
```rust
struct SharedMemoryPool {
    inner: Arc<Mutex<MemoryPool>>,
    local_cache: ThreadLocal<LocalCache>,
}
```

### 2. NUMA-Aware Pooling
```rust
struct NumaAwarePool {
    node_pools: Vec<MemoryPool>,
    current_node: usize,
}
```

### 3. GPU Memory Pooling
```rust
struct GpuMemoryPool {
    cpu_pool: MemoryPool,
    gpu_pool: GpuPool,
    transfer_queue: TransferQueue,
}
```

## Conclusion

This memory pooling design provides a comprehensive framework for reducing allocation overhead in capnproto-rust. The modular architecture allows for gradual adoption and experimentation with different pooling strategies. The expected performance improvements range from 15-50% depending on workload characteristics, with the most significant gains in high-churn scenarios.

The implementation prioritizes safety, maintainability, and backward compatibility while providing substantial performance benefits. The design includes extensive monitoring and metrics to ensure proper behavior and facilitate performance tuning.

**Next Steps:**
1. Implement core pooling infrastructure
2. Integrate with existing allocator system
3. Develop comprehensive test suite
4. Benchmark and optimize performance
5. Document usage patterns and best practices