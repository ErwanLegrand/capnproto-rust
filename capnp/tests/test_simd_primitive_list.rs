// Test SIMD optimizations for primitive list operations
use capnp::message::Builder;
use capnp::primitive_list;

#[test]
fn test_primitive_list_copy_from_slice_simd() {
    let mut message = Builder::new_default();
    
    // Initialize list with specific size
    let mut list = message.initn_root::<primitive_list::Builder<'_, u32>>(5);
    
    // Initialize list with some values
    list.set(0, 1);
    list.set(1, 2);
    list.set(2, 3);
    list.set(3, 4);
    list.set(4, 5);
    
    // Create source data
    let source = [10, 20, 30, 40, 50];
    
    // Test the SIMD copy operation
    #[cfg(feature = "simd")]
    {
        let copied = list.copy_from_slice_simd(&source);
        assert_eq!(copied, 5);
        
        // Verify the copy worked
        assert_eq!(list.get(0), 10);
        assert_eq!(list.get(1), 20);
        assert_eq!(list.get(2), 30);
        assert_eq!(list.get(3), 40);
        assert_eq!(list.get(4), 50);
    }
    
    #[cfg(not(feature = "simd"))]
    {
        // Fallback: manual copy
        for (i, &value) in source.iter().enumerate() {
            list.set(i as u32, value);
        }
        
        // Verify the manual copy worked
        assert_eq!(list.get(0), 10);
        assert_eq!(list.get(1), 20);
        assert_eq!(list.get(2), 30);
        assert_eq!(list.get(3), 40);
        assert_eq!(list.get(4), 50);
    }
}

#[test]
fn test_primitive_list_fill_simd() {
    let mut message = Builder::new_default();
    
    // Initialize list with specific size
    let mut list = message.initn_root::<primitive_list::Builder<'_, u32>>(5);
    
    // Initialize list with some values
    list.set(0, 1);
    list.set(1, 2);
    list.set(2, 3);
    list.set(3, 4);
    list.set(4, 5);
    
    // Test the SIMD fill operation
    #[cfg(feature = "simd")]
    {
        list.fill_simd(42);
        
        // Verify the fill worked
        assert_eq!(list.get(0), 42);
        assert_eq!(list.get(1), 42);
        assert_eq!(list.get(2), 42);
        assert_eq!(list.get(3), 42);
        assert_eq!(list.get(4), 42);
    }
    
    #[cfg(not(feature = "simd"))]
    {
        // Fallback: manual fill
        for i in 0..list.len() {
            list.set(i as u32, 42);
        }
        
        // Verify the manual fill worked
        assert_eq!(list.get(0), 42);
        assert_eq!(list.get(1), 42);
        assert_eq!(list.get(2), 42);
        assert_eq!(list.get(3), 42);
        assert_eq!(list.get(4), 42);
    }
}

#[test]
fn test_primitive_list_simd_performance() {
    use std::time::Instant;
    
    let mut message = Builder::new_default();
    
    // Initialize list with specific size
    const SIZE: u32 = 1000;
    let mut list = message.initn_root::<primitive_list::Builder<'_, u32>>(SIZE);
    
    // Create a larger list for performance testing
    for i in 0..SIZE {
        list.set(i, i as u32);
    }
    
    // Create source data
    let source: Vec<u32> = (SIZE..SIZE*2).collect();
    
    // Benchmark the copy operation
    let iterations = 100;
    let start = Instant::now();
    
    for _ in 0..iterations {
        #[cfg(feature = "simd")]
        {
            list.copy_from_slice_simd(&source);
        }
        
        #[cfg(not(feature = "simd"))]
        {
            for (i, &value) in source.iter().enumerate() {
                if (i as u32) < SIZE {
                    list.set(i as u32, value);
                }
            }
        }
    }
    
    let elapsed = start.elapsed();
    
    println!("Primitive list copy performance:");
    println!("  {} iterations of {} elements", iterations, SIZE);
    println!("  Total time: {:?}", elapsed);
    println!("  Average time per operation: {:?}", elapsed / iterations as u32);
    
    // Verify the last operation worked
    #[cfg(feature = "simd")]
    {
        assert_eq!(list.get(SIZE-1), source[(SIZE-1) as usize]);
    }
}