//! Memory pooling implementation

use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::message::{Allocator, HeapAllocator};
use crate::{Error, Result};

/// Memory usage statistics for a pooled segment
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryUsage {
    pub resident: usize,
    pub virtual_: usize,
    pub heap: usize,
}

/// CPU usage statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuUsage {
    pub user: f64,
    pub system: f64,
    pub total: f64,
}

/// Configuration for memory pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of segments to cache in the pool
    pub max_pool_size: usize,
    
    /// Segment size for fixed-size pooling (if used)
    pub fixed_segment_size: Option<u32>,
    
    /// Size classes for variable-size pooling (if used)
    pub size_classes: Option<Vec<u32>>,
    
    /// Maximum segment age before cleanup
    pub max_segment_age: std::time::Duration,
    
    /// Whether to reset allocator state on deallocation
    pub reset_on_deallocate: bool,
    
    /// Enable adaptive pooling strategy
    pub adaptive: bool,
    
    /// Enable thread-local pooling
    pub thread_local: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 1024,
            fixed_segment_size: Some(1024),
            size_classes: None,
            max_segment_age: std::time::Duration::from_secs(60),
            reset_on_deallocate: true,
            adaptive: false,
            thread_local: false,
        }
    }
}

/// Metrics for monitoring memory pool performance
#[derive(Debug, Clone, Default)]
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
    
    /// Allocation latency statistics (in microseconds)
    pub allocation_latency: Vec<u64>,
    
    /// Fragmentation metrics
    pub fragmentation_ratio: f32,
}

impl PoolMetrics {
    /// Calculate the pool hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.pool_hits + self.pool_misses;
        if total == 0 {
            0.0
        } else {
            self.pool_hits as f32 / total as f32
        }
    }
    
    /// Calculate memory savings from pooling
    pub fn memory_savings(&self) -> usize {
        self.bytes_saved
    }
    
    /// Calculate average allocation latency
    pub fn avg_allocation_latency(&self) -> f64 {
        if self.allocation_latency.is_empty() {
            0.0
        } else {
            let sum: u64 = self.allocation_latency.iter().sum();
            sum as f64 / self.allocation_latency.len() as f64
        }
    }
}

/// Error types for memory pooling operations
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Memory pool exhausted")]
    PoolExhausted,
    
    #[error("Invalid segment pointer")]
    InvalidSegment,
    
    #[error("Segment size mismatch")]
    SizeMismatch,
    
    #[error("Alignment error: pointer must be 8-byte aligned")]
    AlignmentError,
    
    #[error("Pool capacity exceeded: {0}")]
    CapacityExceeded(usize),
    
    #[error("Segment too large for pool: {0} words")]
    SegmentTooLarge(u32),
}

/// Pooled segment information
#[derive(Debug)]
struct PooledSegment {
    ptr: *mut u8,
    size: u32,
    last_used: Instant,
    generation: u64,
}

impl PooledSegment {
    fn new(ptr: *mut u8, size: u32) -> Self {
        Self {
            ptr,
            size,
            last_used: Instant::now(),
            generation: 0,
        }
    }
}

/// Fixed-size memory pool
#[cfg(feature = "memory-pooling")]
pub struct FixedSizePool {
    segment_size: u32,
    free_segments: Vec<PooledSegment>,
    capacity: usize,
    metrics: PoolMetrics,
    generation_counter: AtomicU64,
}

#[cfg(feature = "memory-pooling")]
impl FixedSizePool {
    /// Create a new fixed-size pool
    pub fn new(segment_size: u32, capacity: usize) -> Self {
        Self {
            segment_size,
            free_segments: Vec::with_capacity(capacity),
            capacity,
            metrics: PoolMetrics::default(),
            generation_counter: AtomicU64::new(0),
        }
    }
    
    /// Allocate a segment from the pool or fallback to heap
    pub fn allocate_segment(&mut self, size: u32) -> Result<(*mut u8, u32)> {
        // If size is larger than our pool size, allocate directly
        if size > self.segment_size {
            self.metrics.pool_misses += 1;
            let mut fallback = HeapAllocator::new();
            return Ok(fallback.allocate_segment(size));
        }
        
        // Try to get a segment from the pool
        if let Some(segment) = self.free_segments.pop() {
            self.metrics.pool_hits += 1;
            self.metrics.current_pool_size -= 1;
            Ok((segment.ptr, segment.size))
        } else {
            // Pool is empty, allocate new segment
            self.metrics.pool_misses += 1;
            let mut fallback = HeapAllocator::new();
            let (ptr, size) = fallback.allocate_segment(self.segment_size);
            Ok((ptr, size))
        }
    }
    
    /// Deallocate a segment back to the pool
    pub fn deallocate_segment(&mut self, ptr: *mut u8, size: u32) -> Result<()> {
        // Validate the segment
        self.validate_segment(ptr, size)?;
        
        // Only accept segments of our fixed size
        if size != self.segment_size {
            // Deallocate through heap instead
            let mut fallback = HeapAllocator::new();
            unsafe { fallback.deallocate_segment(ptr, size, size) };
            return Ok(());
        }
        
        // Check if pool has capacity
        if self.free_segments.len() >= self.capacity {
            // Pool is full, deallocate through heap
            let mut fallback = HeapAllocator::new();
            unsafe { fallback.deallocate_segment(ptr, size, size) };
            self.metrics.bytes_saved += size as usize * 8; // 8 bytes per word
            return Ok(());
        }
        
        // Add to pool
        let _generation = self.generation_counter.fetch_add(1, Ordering::Relaxed);
        let segment = PooledSegment::new(ptr, size);
        self.free_segments.push(segment);
        self.metrics.current_pool_size += 1;
        self.metrics.max_pool_size = self.metrics.max_pool_size.max(self.free_segments.len());
        
        Ok(())
    }
    
    /// Validate a segment pointer and size
    fn validate_segment(&self, ptr: *mut u8, size: u32) -> Result<()> {
        if ptr.is_null() {
            return Err(Error::failed("Invalid segment pointer".to_string()));
        }
        
        // Check alignment
        if (ptr as usize) % 8 != 0 {
            return Err(Error::failed("Alignment error: pointer must be 8-byte aligned".to_string()));
        }
        
        // Check size is reasonable
        if size == 0 || size > 1 << 20 { // Max 1MB segment
            return Err(Error::failed(format!("Segment too large: {} words", size)));
        }
        
        Ok(())
    }
    
    /// Clean up old segments
    pub fn cleanup_old_segments(&mut self, max_age: std::time::Duration) {
        let now = Instant::now();
        self.free_segments.retain(|segment| {
            now.duration_since(segment.last_used) <= max_age
        });
        self.metrics.current_pool_size = self.free_segments.len();
    }
    
    /// Get current metrics
    pub fn get_metrics(&self) -> PoolMetrics {
        self.metrics.clone()
    }
}

/// Variable-size memory pool with size classes
#[cfg(feature = "memory-pooling")]
pub struct VariableSizePool {
    size_classes: Vec<u32>,
    pools: Vec<FixedSizePool>,
    metrics: PoolMetrics,
}

#[cfg(feature = "memory-pooling")]
impl VariableSizePool {
    /// Create a new variable-size pool with specified size classes
    pub fn new(size_classes: Vec<u32>, capacity_per_class: usize) -> Self {
        let pools = size_classes.iter()
            .map(|&size| FixedSizePool::new(size, capacity_per_class))
            .collect();
        
        Self {
            size_classes,
            pools,
            metrics: PoolMetrics::default(),
        }
    }
    
    /// Find the appropriate size class for a given size
    fn find_size_class(&self, size: u32) -> Option<usize> {
        self.size_classes.iter()
            .position(|&class_size| size <= class_size)
    }
    
    /// Allocate a segment from the appropriate size class
    pub fn allocate_segment(&mut self, size: u32) -> Result<(*mut u8, u32)> {
        if let Some(class_idx) = self.find_size_class(size) {
            self.pools[class_idx].allocate_segment(size)
        } else {
            // Too large for any size class, allocate directly
            self.metrics.pool_misses += 1;
            let mut fallback = HeapAllocator::new();
            Ok(fallback.allocate_segment(size))
        }
    }
    
    /// Deallocate a segment to the appropriate size class
    pub fn deallocate_segment(&mut self, ptr: *mut u8, size: u32) -> Result<()> {
        if let Some(class_idx) = self.find_size_class(size) {
            self.pools[class_idx].deallocate_segment(ptr, size)
        } else {
            // Too large for any size class, deallocate directly
            let mut fallback = HeapAllocator::new();
            unsafe { fallback.deallocate_segment(ptr, size, size) };
            Ok(())
        }
    }
    
    /// Clean up old segments in all size classes
    pub fn cleanup_old_segments(&mut self, max_age: std::time::Duration) {
        for pool in &mut self.pools {
            pool.cleanup_old_segments(max_age);
        }
    }
    
    /// Get current metrics
    pub fn get_metrics(&self) -> PoolMetrics {
        // Aggregate metrics from all size classes
        let mut metrics = PoolMetrics::default();
        
        for pool in &self.pools {
            let pool_metrics = pool.get_metrics();
            metrics.pool_hits += pool_metrics.pool_hits;
            metrics.pool_misses += pool_metrics.pool_misses;
            metrics.current_pool_size += pool_metrics.current_pool_size;
            metrics.max_pool_size = metrics.max_pool_size.max(pool_metrics.max_pool_size);
            metrics.bytes_managed += pool_metrics.bytes_managed;
            metrics.bytes_saved += pool_metrics.bytes_saved;
            metrics.allocation_latency.extend(pool_metrics.allocation_latency);
        }
        
        metrics
    }
}

/// Main memory pool structure
pub struct MemoryPool {
    strategy: PoolingStrategy,
    fallback_allocator: HeapAllocator,
    metrics: PoolMetrics,
    config: PoolConfig,
}

/// Available pooling strategies
pub enum PoolingStrategy {
    /// Fixed-size pooling (simple and efficient)
    FixedSize(#[cfg(feature = "memory-pooling")] FixedSizePool),
    
    /// Variable-size pooling with size classes
    VariableSize(#[cfg(feature = "memory-pooling")] VariableSizePool),
    
    /// No pooling (fallback to heap allocator)
    None,
}

impl MemoryPool {
    /// Create a new memory pool with default configuration
    pub fn new_default() -> Self {
        let config = PoolConfig::default();
        Self::new_with_config(config)
    }
    
    /// Create a new memory pool with fixed-size strategy
    #[cfg(feature = "memory-pooling")]
    pub fn new_fixed_size(segment_size: u32, capacity: usize) -> Self {
        let mut config = PoolConfig::default();
        config.fixed_segment_size = Some(segment_size);
        
        let strategy = PoolingStrategy::FixedSize(FixedSizePool::new(segment_size, capacity));
        
        Self {
            strategy,
            fallback_allocator: HeapAllocator::new(),
            metrics: PoolMetrics::default(),
            config,
        }
    }
    
    /// Create a new memory pool with variable-size strategy
    #[cfg(feature = "memory-pooling")]
    pub fn new_variable_size(size_classes: Vec<u32>, capacity_per_class: usize) -> Self {
        let mut config = PoolConfig::default();
        config.size_classes = Some(size_classes.clone());
        
        let strategy = PoolingStrategy::VariableSize(
            VariableSizePool::new(size_classes, capacity_per_class)
        );
        
        Self {
            strategy,
            fallback_allocator: HeapAllocator::new(),
            metrics: PoolMetrics::default(),
            config,
        }
    }
    
    /// Create a memory pool with custom configuration
    pub fn new_with_config(config: PoolConfig) -> Self {
        #[cfg(feature = "memory-pooling")]
        let strategy = if let Some(segment_size) = config.fixed_segment_size {
            PoolingStrategy::FixedSize(FixedSizePool::new(segment_size, config.max_pool_size))
        } else if let Some(ref size_classes) = config.size_classes {
            PoolingStrategy::VariableSize(
                VariableSizePool::new(size_classes.clone(), config.max_pool_size)
            )
        } else {
            PoolingStrategy::None
        };
        
        #[cfg(not(feature = "memory-pooling"))]
        let strategy = PoolingStrategy::None;
        
        Self {
            strategy,
            fallback_allocator: HeapAllocator::new(),
            metrics: PoolMetrics::default(),
            config,
        }
    }
    
    /// Get current pool metrics
    pub fn get_metrics(&self) -> PoolMetrics {
        match &self.strategy {
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::FixedSize(pool) => pool.get_metrics(),
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::VariableSize(pool) => pool.get_metrics(),
            PoolingStrategy::None => self.metrics.clone(),
        }
    }
    
    /// Clean up old segments based on age
    pub fn cleanup_old_segments(&mut self) {
        match &mut self.strategy {
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::FixedSize(pool) => {
                pool.cleanup_old_segments(self.config.max_segment_age);
            }
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::VariableSize(pool) => {
                pool.cleanup_old_segments(self.config.max_segment_age);
            }
            PoolingStrategy::None => {}
        }
    }
}

/// Implement Allocator trait for MemoryPool
unsafe impl Allocator for MemoryPool {
    fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
        match &mut self.strategy {
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::FixedSize(pool) => {
                match pool.allocate_segment(minimum_size) {
                    Ok(segment) => segment,
                    Err(_) => self.fallback_allocator.allocate_segment(minimum_size),
                }
            }
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::VariableSize(pool) => {
                match pool.allocate_segment(minimum_size) {
                    Ok(segment) => segment,
                    Err(_) => self.fallback_allocator.allocate_segment(minimum_size),
                }
            }
            PoolingStrategy::None => {
                self.fallback_allocator.allocate_segment(minimum_size)
            }
        }
    }
    
    unsafe fn deallocate_segment(&mut self, ptr: *mut u8, word_size: u32, words_used: u32) {
        match &mut self.strategy {
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::FixedSize(pool) => {
                if let Err(_) = pool.deallocate_segment(ptr, word_size) {
                    self.fallback_allocator.deallocate_segment(ptr, word_size, words_used);
                }
            }
            #[cfg(feature = "memory-pooling")]
            PoolingStrategy::VariableSize(pool) => {
                if let Err(_) = pool.deallocate_segment(ptr, word_size) {
                    self.fallback_allocator.deallocate_segment(ptr, word_size, words_used);
                }
            }
            PoolingStrategy::None => {
                self.fallback_allocator.deallocate_segment(ptr, word_size, words_used);
            }
        }
        
        // Reset allocator state if configured
        if self.config.reset_on_deallocate {
            self.fallback_allocator = HeapAllocator::new();
        }
    }
}

/// Extension trait for adding pooling to Builder
pub trait BuilderPoolingExt {
    /// Create a builder with memory pooling
    fn new_with_pooling(pool: MemoryPool) -> Self;
}

impl BuilderPoolingExt for crate::message::Builder<MemoryPool> {
    fn new_with_pooling(pool: MemoryPool) -> Self {
        Self::new(pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_size_pool_allocation() {
        let mut pool = MemoryPool::new_fixed_size(1024, 10);
        
        // Test allocation
        let (ptr, size) = pool.allocate_segment(512);
        assert_eq!(size, 1024); // Should round up to pool size
        
        // Test deallocation
        unsafe { pool.deallocate_segment(ptr, size, 512) };
        
        // Test reuse
        let (ptr2, size2) = pool.allocate_segment(256);
        assert_eq!(size2, 1024);
        
        let metrics = pool.get_metrics();
        assert_eq!(metrics.pool_hits, 1);
        assert_eq!(metrics.pool_misses, 1);
    }
    
    #[test]
    fn test_variable_size_pool() {
        let size_classes = vec![256, 512, 1024];
        let mut pool = MemoryPool::new_variable_size(size_classes, 5);
        
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
        
        let metrics = pool.get_metrics();
        assert_eq!(metrics.pool_hits, 1);
    }
    
    #[test]
    fn test_pool_metrics() {
        let mut pool = MemoryPool::new_fixed_size(512, 5);
        
        // Make some allocations and deallocations
        for _ in 0..3 {
            let (ptr, _) = pool.allocate_segment(256);
            unsafe { pool.deallocate_segment(ptr, 512, 256) };
        }
        
        let metrics = pool.get_metrics();
        assert!(metrics.pool_hits > 0);
        assert!(metrics.pool_misses > 0);
        assert!(metrics.hit_rate() > 0.0);
    }
}