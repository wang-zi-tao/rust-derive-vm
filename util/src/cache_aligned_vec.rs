use std::mem::size_of;

use smallvec::SmallVec;

pub type CacheAlignedVec<T> = SmallVec<[T; (64 - size_of::<usize>()) / size_of::<T>()]>;
