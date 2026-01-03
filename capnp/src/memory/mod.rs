//! Memory pooling module for Cap'n Proto
//!
//! This module provides memory pooling capabilities to reduce allocation overhead
//! in performance-critical applications.

pub mod pool;

pub use pool::{MemoryPool, PoolConfig, PoolMetrics, PoolError, PoolingStrategy};

#[cfg(feature = "memory-pooling")]
pub use pool::{FixedSizePool, VariableSizePool};
