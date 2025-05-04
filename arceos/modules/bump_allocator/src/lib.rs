#![no_std]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::ptr::NonNull;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
        }
    }
}

#[inline]
const fn align_down(pos: usize, align: usize) -> usize {
    pos & !(align - 1)
}

#[inline]
const fn align_up(pos: usize, align: usize) -> usize {
    (pos + align - 1) & !(align - 1)
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    /// Initialize the allocator with a free memory region.
    fn init(&mut self, start: usize, size: usize) {
        assert!(PAGE_SIZE.is_power_of_two());
        self.start = align_down(start, PAGE_SIZE);
        self.end = align_up(start + size, PAGE_SIZE);
        self.b_pos = self.start;
        self.p_pos = self.end;
    }

    /// Add a free memory region to the allocator.
    fn add_memory(&mut self, _start: usize, _size: usize) -> AllocResult {
        Err(AllocError::NoMemory) // unsupported
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    /// Allocate memory with the given size (in bytes) and alignment.
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let layout = layout.pad_to_align();
        let new_b_pos = self.b_pos + layout.size();
        if new_b_pos > self.p_pos {
            Err(AllocError::NoMemory)
        } else {
            let ptr = unsafe { NonNull::new_unchecked(self.b_pos as *mut u8) };
            Ok(ptr)
        }
    }

    /// Deallocate memory at the given position, size, and alignment.
    /// arena, everything will be fine after the early stage end
    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {}

    /// Returns total memory size in bytes.
    fn total_bytes(&self) -> usize {
        self.p_pos - self.start
    }

    /// Returns allocated memory size in bytes.
    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    /// Returns available memory size in bytes.
    fn available_bytes(&self) -> usize {
        self.total_bytes() - self.used_bytes()
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    /// The size of a memory page.
    const PAGE_SIZE: usize = PAGE_SIZE;

    /// Allocate contiguous memory pages with given count and alignment.
    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if align_pow2 % PAGE_SIZE != 0 {
            return Err(AllocError::InvalidParam);
        }
        let align_pow2 = align_pow2 / PAGE_SIZE;
        if !align_pow2.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }
        let remain = align_pow2 - (num_pages % align_pow2);
        let num_pages = num_pages + remain;
        let new_p_pos = self.p_pos - num_pages * PAGE_SIZE;
        if new_p_pos < self.b_pos {
            Err(AllocError::NoMemory)
        } else {
            self.p_pos = new_p_pos;
            Ok(self.p_pos)
        }
    }

    /// Deallocate contiguous memory pages with given position and count.
    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // arena attempt
        unimplemented!();
    }

    /// Returns the total number of memory pages.
    fn total_pages(&self) -> usize {
        (self.end - self.b_pos) / PAGE_SIZE // maximum available pages number
    }

    /// Returns the number of allocated memory pages.
    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / PAGE_SIZE
    }

    /// Returns the number of available memory pages.
    fn available_pages(&self) -> usize {
        self.total_pages() - self.used_pages()
    }
}
