// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Raw-erased whole-batch segment transport.

use std::iter;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::scheduler::{Id, Value};

#[cfg(test)]
use super::InputStorage;
use super::{Change, InputChange, InputValue, Port};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

enum SegmentStorage {
    Unique {
        buffer: ManuallyDrop<Buffer>,
        cursor: usize,
    },
    Owned {
        allocation: Arc<Allocation>,
        start: usize,
        cursor: usize,
        end: usize,
        vtable: SegmentVTable,
    },
    Shared {
        buffer: Arc<SharedRange>,
        start: usize,
        cursor: usize,
        end: usize,
        vtable: SegmentVTable,
    },
}

// ----------------------------------------------------------------------------

enum FanOutState<I> {
    Empty,
    Single(Option<Segment<I>>),
    Shared {
        buffer: Option<Arc<SharedRange>>,
        start: usize,
        end: usize,
        vtable: SegmentVTable,
    },
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct SegmentVTable {
    port: Port,
    drop_range: unsafe fn(NonNull<u8>, usize, usize),
    deallocate: unsafe fn(NonNull<u8>, usize),
}

// ----------------------------------------------------------------------------

struct Buffer {
    pointer: NonNull<u8>,
    length: usize,
    capacity: usize,
    vtable: SegmentVTable,
}

// ----------------------------------------------------------------------------

struct Allocation {
    pointer: NonNull<u8>,
    capacity: usize,
    deallocate: unsafe fn(NonNull<u8>, usize),
}

// ----------------------------------------------------------------------------

struct SharedRange {
    allocation: Arc<Allocation>,
    start: usize,
    end: usize,
    drop_range: unsafe fn(NonNull<u8>, usize, usize),
}

// ----------------------------------------------------------------------------

/// One raw-erased, whole-batch transport segment.
///
/// Unique segments move items directly. Fan-out creates immutable shared
/// storage with one cursor per lease; the final lease can recover ownership.
// Public only within this private module so the sealed `Inputs` trait can name
// it without exposing raw transport through the action API.
pub struct Segment<I> {
    storage: Option<SegmentStorage>,
    marker: PhantomData<fn() -> I>,
}

// ----------------------------------------------------------------------------

/// Owning iterator over independently consumable segment leases.
///
/// The iterator is intentionally allocation-free for zero or one subscriber.
/// True fan-out allocates only the shared ownership block required by the
/// leases themselves.
#[must_use]
pub(crate) struct FanOut<I> {
    state: FanOutState<I>,
    remaining: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Buffer {
    fn new<I, V>(items: Vec<Change<I, V>>) -> Self
    where
        I: Id,
        V: Value,
    {
        let mut items = ManuallyDrop::new(items);
        let pointer = NonNull::new(items.as_mut_ptr().cast::<u8>())
            .expect("Vec pointers are non-null");
        Self {
            pointer,
            length: items.len(),
            capacity: items.capacity(),
            vtable: SegmentVTable {
                port: Port::of::<I, V>(),
                drop_range: drop_range::<Change<I, V>>,
                deallocate: deallocate::<Change<I, V>>,
            },
        }
    }

    unsafe fn drop_from(&mut self, cursor: usize) {
        let allocation = Allocation {
            pointer: self.pointer,
            capacity: self.capacity,
            deallocate: self.vtable.deallocate,
        };
        // SAFETY: the caller identifies precisely the initialized unread range.
        // Slice drop glue continues dropping remaining elements during unwind;
        // the allocation guard then always releases the backing buffer.
        unsafe {
            (self.vtable.drop_range)(self.pointer, cursor, self.length);
        }
        drop(allocation);
        self.length = 0;
        self.capacity = 0;
    }
}

// ----------------------------------------------------------------------------

impl<I> Segment<I>
where
    I: Id,
{
    /// Erases an owned typed item vector without allocating a wrapper.
    #[must_use]
    pub(crate) fn new<V>(items: Vec<Change<I, V>>) -> Self
    where
        V: Value,
    {
        Self {
            storage: Some(SegmentStorage::Unique {
                buffer: ManuallyDrop::new(Buffer::new(items)),
                cursor: 0,
            }),
            marker: PhantomData,
        }
    }

    /// Returns the exact typed port carried by this segment.
    ///
    /// # Panics
    ///
    /// Panics only if an internal ownership transition left the segment empty.
    #[must_use]
    pub(crate) fn port(&self) -> Port {
        storage_port(self.storage.as_ref().expect("segment owns storage"))
    }

    /// Creates one independently consumable lease per subscriber.
    ///
    /// # Panics
    ///
    /// Panics when asked to fan out a partially consumed segment. Fan-out is
    /// defined only for newly sealed output.
    pub(crate) fn fan_out(mut self, subscribers: usize) -> FanOut<I> {
        if subscribers == 0 {
            drop(self);
            return FanOut {
                state: FanOutState::Empty,
                remaining: 0,
            };
        }
        if subscribers == 1 {
            return FanOut {
                state: FanOutState::Single(Some(self)),
                remaining: 1,
            };
        }
        if !storage_is_sealed(
            self.storage.as_ref().expect("segment owns storage"),
        ) {
            panic!("only sealed owned segments may fan out");
        }
        self.promote_to_owned_range();
        let storage = self.storage.take().expect("segment owns storage");
        let (buffer, start, end, vtable) = match storage {
            SegmentStorage::Owned {
                allocation,
                start,
                cursor,
                end,
                vtable,
            } => {
                debug_assert_eq!(start, cursor);
                let buffer = Arc::new(SharedRange {
                    allocation,
                    start,
                    end,
                    drop_range: vtable.drop_range,
                });
                (buffer, start, end, vtable)
            }
            SegmentStorage::Shared {
                buffer,
                start,
                cursor,
                end,
                vtable,
            } => {
                debug_assert_eq!(start, cursor);
                (buffer, start, end, vtable)
            }
            SegmentStorage::Unique { .. } => {
                unreachable!("unique segment became an owning range")
            }
        };
        // Do not collect these leases. The scheduler consumes them alongside
        // affine destination reservations, and staging a Vec adds one
        // allocation to every emitting action invocation.
        FanOut {
            state: FanOutState::Shared {
                buffer: Some(buffer),
                start,
                end,
                vtable,
            },
            remaining: subscribers,
        }
    }

    /// Splits off at most `limit` unread items without copying them.
    pub(crate) fn split_prefix(mut self, limit: usize) -> (Self, Option<Self>) {
        assert!(limit != 0, "segment slices must contain at least one item");
        if self.len() <= limit {
            return (self, None);
        }
        self.promote_to_owned_range();
        let storage = self.storage.take().expect("segment owns storage");
        let (prefix, tail) = match storage {
            SegmentStorage::Owned {
                allocation,
                start,
                cursor,
                end,
                vtable,
            } => {
                let middle = cursor
                    .checked_add(limit)
                    .expect("segment range overflowed");
                debug_assert!(middle < end);
                let tail = SegmentStorage::Owned {
                    allocation: Arc::clone(&allocation),
                    start: middle,
                    cursor: middle,
                    end,
                    vtable,
                };
                let prefix = SegmentStorage::Owned {
                    allocation,
                    start,
                    cursor,
                    end: middle,
                    vtable,
                };
                (prefix, tail)
            }
            SegmentStorage::Shared {
                buffer,
                start,
                cursor,
                end,
                vtable,
            } => {
                let middle = cursor
                    .checked_add(limit)
                    .expect("segment range overflowed");
                debug_assert!(middle < end);
                let tail = SegmentStorage::Shared {
                    buffer: Arc::clone(&buffer),
                    start: middle,
                    cursor: middle,
                    end,
                    vtable,
                };
                let prefix = SegmentStorage::Shared {
                    buffer,
                    start,
                    cursor,
                    end: middle,
                    vtable,
                };
                (prefix, tail)
            }
            SegmentStorage::Unique { .. } => {
                unreachable!("unique segment became an owning range")
            }
        };
        self.storage = Some(prefix);
        (
            self,
            Some(Self {
                storage: Some(tail),
                marker: PhantomData,
            }),
        )
    }

    fn promote_to_owned_range(&mut self) {
        if !matches!(self.storage, Some(SegmentStorage::Unique { .. })) {
            return;
        }
        let Some(SegmentStorage::Unique { buffer, cursor }) =
            self.storage.take()
        else {
            unreachable!();
        };
        let length = buffer.length;
        let vtable = buffer.vtable;
        let allocation = Arc::new(Allocation {
            pointer: buffer.pointer,
            capacity: buffer.capacity,
            deallocate: buffer.vtable.deallocate,
        });
        self.storage = Some(SegmentStorage::Owned {
            allocation,
            start: cursor,
            cursor,
            end: length,
            vtable,
        });
    }

    pub(super) fn promote_if_unique(&mut self) {
        let Some(SegmentStorage::Shared {
            buffer,
            start,
            cursor,
            end,
            vtable: _,
        }) = self.storage.as_ref()
        else {
            return;
        };
        if cursor != start || *start != buffer.start || *end != buffer.end {
            return;
        }
        let storage = self.storage.take().expect("segment owns storage");
        let SegmentStorage::Shared {
            buffer,
            start,
            cursor,
            end,
            vtable,
        } = storage
        else {
            unreachable!();
        };
        self.storage = Some(match Arc::try_unwrap(buffer) {
            Ok(buffer) => {
                let buffer = ManuallyDrop::new(buffer);
                // SAFETY: ManuallyDrop suppresses SharedRange drop while its
                // exact allocation ownership moves into the owned range.
                let allocation =
                    unsafe { std::ptr::read(&raw const buffer.allocation) };
                SegmentStorage::Owned {
                    allocation,
                    start,
                    cursor,
                    end,
                    vtable,
                }
            }
            Err(buffer) => SegmentStorage::Shared {
                buffer,
                start,
                cursor,
                end,
                vtable,
            },
        });
    }

    pub(crate) fn len(&self) -> usize {
        match self.storage.as_ref().expect("segment owns storage") {
            SegmentStorage::Unique { buffer, cursor } => {
                buffer.length - *cursor
            }
            SegmentStorage::Owned { cursor, end, .. }
            | SegmentStorage::Shared { cursor, end, .. } => end - cursor,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn drain<V>(
        mut self, mut callback: impl FnMut(InputChange<'_, I, V>),
    ) where
        V: Value,
    {
        // Unlike action inputs, the external caller selects V at this boundary.
        assert_eq!(self.port(), Port::of::<I, V>(), "egress port mismatch");
        self.promote_if_unique();
        loop {
            let Some(change) = self.pop_front::<V>() else {
                break;
            };
            callback(change);
        }
    }

    pub(crate) fn pop_front_owned<V>(&mut self) -> Option<Change<I, V>>
    where
        V: Value,
    {
        self.promote_if_unique();
        let change = self.pop_front::<V>()?;
        let change = match change {
            Change::Insert(key, value) => {
                Change::Insert(key, value.into_owned())
            }
            Change::Remove(key) => Change::Remove(key),
        };
        Some(change)
    }

    pub(super) fn pop_front<V>(&mut self) -> Option<InputChange<'_, I, V>>
    where
        V: Value,
    {
        // Plan validation proves this internal binding. Egress performs its
        // own release assertion before entering this loop.
        debug_assert_eq!(self.port(), Port::of::<I, V>());
        let storage = self.storage.as_mut().expect("segment owns storage");
        match storage {
            SegmentStorage::Unique { buffer, cursor } => {
                if *cursor == buffer.length {
                    return None;
                }
                let index = *cursor;
                *cursor += 1;
                // SAFETY: the validated port identifies Change<I, V>, and every
                // unique index is read once before Drop skips the prefix.
                let item = unsafe {
                    buffer
                        .pointer
                        .cast::<Change<I, V>>()
                        .as_ptr()
                        .add(index)
                        .read()
                };
                Some(own_change(item))
            }
            SegmentStorage::Owned { allocation, cursor, end, .. } => {
                if *cursor == *end {
                    return None;
                }
                let index = *cursor;
                *cursor += 1;
                // SAFETY: the validated port identifies Change<I, V>. Disjoint
                // owning ranges are created only by the contained split
                // protocol, and each index is moved once.
                let item = unsafe {
                    allocation
                        .pointer
                        .cast::<Change<I, V>>()
                        .as_ptr()
                        .add(index)
                        .read()
                };
                Some(own_change(item))
            }
            SegmentStorage::Shared { buffer, cursor, end, .. } => {
                if *cursor == *end {
                    return None;
                }
                let index = *cursor;
                *cursor += 1;
                // SAFETY: the validated port identifies Change<I, V>, and shared
                // payload remains immutable and Arc-owned for this borrow.
                let item = unsafe {
                    &*buffer
                        .allocation
                        .pointer
                        .cast::<Change<I, V>>()
                        .as_ptr()
                        .add(index)
                };
                Some(borrow_change(item))
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: this guard owns the original allocation after all initialized
        // elements have entered drop glue.
        unsafe { (self.deallocate)(self.pointer, self.capacity) };
    }
}

// SAFETY: construction requires the erased item type to be Send + Sync.
unsafe impl Send for Allocation {}

// SAFETY: an allocation exposes items only through its owning range protocol.
unsafe impl Sync for Allocation {}

// ----------------------------------------------------------------------------

impl Drop for SharedRange {
    fn drop(&mut self) {
        // SAFETY: this shared range owns drop responsibility for every item in
        // its exact range. Individual leases only borrow those items.
        unsafe {
            (self.drop_range)(self.allocation.pointer, self.start, self.end);
        }
    }
}

// ----------------------------------------------------------------------------

// SAFETY: construction requires the erased item type to be Send + Sync.
unsafe impl Send for Buffer {}

// SAFETY: shared buffers only yield immutable references.
unsafe impl Sync for Buffer {}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: shared buffers never move individual items.
        unsafe { self.drop_from(0) };
    }
}

// ----------------------------------------------------------------------------

impl<I> Iterator for FanOut<I> {
    type Item = Segment<I>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        match &mut self.state {
            FanOutState::Empty => unreachable!("non-empty fan-out is empty"),
            FanOutState::Single(segment) => segment.take(),
            FanOutState::Shared { buffer, start, end, vtable } => {
                let buffer = if self.remaining == 0 {
                    buffer.take().expect("shared fan-out retains its buffer")
                } else {
                    Arc::clone(
                        buffer
                            .as_ref()
                            .expect("shared fan-out retains its buffer"),
                    )
                };
                Some(Segment {
                    storage: Some(SegmentStorage::Shared {
                        buffer,
                        start: *start,
                        cursor: *start,
                        end: *end,
                        vtable: *vtable,
                    }),
                    marker: PhantomData,
                })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<I> ExactSizeIterator for FanOut<I> {}

impl<I> iter::FusedIterator for FanOut<I> {}

// ----------------------------------------------------------------------------

impl<I> Drop for Segment<I> {
    fn drop(&mut self) {
        let Some(storage) = self.storage.take() else {
            return;
        };
        match storage {
            SegmentStorage::Unique { mut buffer, cursor } => {
                // SAFETY: items before cursor were moved or explicitly
                // destroyed.
                unsafe { buffer.drop_from(cursor) };
            }
            SegmentStorage::Owned {
                allocation,
                cursor,
                end,
                vtable,
                ..
            } => {
                // SAFETY: this disjoint owning range alone is responsible for
                // dropping its unread items.
                unsafe {
                    (vtable.drop_range)(allocation.pointer, cursor, end);
                }
            }
            SegmentStorage::Shared { .. } => {}
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn storage_port(storage: &SegmentStorage) -> Port {
    match storage {
        SegmentStorage::Unique { buffer, .. } => buffer.vtable.port,
        SegmentStorage::Owned { vtable, .. }
        | SegmentStorage::Shared { vtable, .. } => vtable.port,
    }
}

fn storage_is_sealed(storage: &SegmentStorage) -> bool {
    match storage {
        SegmentStorage::Unique { cursor, .. } => *cursor == 0,
        SegmentStorage::Owned { start, cursor, .. }
        | SegmentStorage::Shared { start, cursor, .. } => start == cursor,
    }
}

fn own_change<I, V>(change: Change<I, V>) -> InputChange<'static, I, V> {
    match change {
        Change::Insert(key, value) => {
            Change::Insert(key, InputValue::owned(value))
        }
        Change::Remove(key) => Change::Remove(key),
    }
}

fn borrow_change<I, V>(change: &Change<I, V>) -> InputChange<'_, I, V>
where
    I: Id,
{
    match change {
        Change::Insert(key, value) => {
            Change::Insert(key.clone(), InputValue::borrowed(value))
        }
        Change::Remove(key) => Change::Remove(key.clone()),
    }
}

unsafe fn drop_range<T>(pointer: NonNull<u8>, start: usize, end: usize) {
    // SAFETY: this range contains initialized T values. Slice drop glue also
    // drops the rest of the range if one destructor unwinds.
    unsafe {
        std::ptr::slice_from_raw_parts_mut(
            pointer.cast::<T>().as_ptr().add(start),
            end - start,
        )
        .drop_in_place();
    }
}

unsafe fn deallocate<T>(pointer: NonNull<u8>, capacity: usize) {
    // SAFETY: these are the original Vec raw parts with no live elements.
    unsafe {
        drop(Vec::from_raw_parts(
            pointer.cast::<T>().as_ptr(),
            0,
            capacity,
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Change, InputStorage, Port, Segment};
    use crate::scheduler::Value;

    struct MoveOnly(String);

    impl Clone for MoveOnly {
        fn clone(&self) -> Self {
            panic!("move-only test value was cloned")
        }
    }

    #[derive(Clone)]
    struct DropCount(Arc<AtomicUsize>);

    impl Drop for DropCount {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[derive(Clone)]
    struct PanicDrop {
        drops: Arc<AtomicUsize>,
        panic: bool,
    }

    impl Value for MoveOnly {}

    impl Value for DropCount {}

    impl Value for PanicDrop {}

    impl Drop for PanicDrop {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
            assert!(!self.panic, "drop panic");
        }
    }

    fn item<V>(key: u64, value: V) -> Change<u64, V> {
        Change::Insert(key, value)
    }

    #[test]
    fn raw_segment_reports_exact_batch_port() {
        let segment = Segment::new(vec![item(1, String::from("one"))]);
        assert_eq!(segment.port(), Port::of::<u64, String>());
    }

    #[test]
    fn raw_segment_accepts_a_hierarchical_complete_identity() {
        let identity = vec![1_u64, 2];
        let mut segment =
            Segment::new(vec![Change::Insert(identity.clone(), 10_u64)]);
        assert_eq!(segment.port(), Port::of::<Vec<u64>, u64>());

        let change = segment.pop_front::<u64>().unwrap();
        let Change::Insert(actual, _) = change else {
            panic!("unexpected removal");
        };
        assert_eq!(actual, identity);
    }

    #[test]
    fn raw_segment_moves_unique_value_without_cloning() {
        let mut segment = Segment::new(vec![item(1, MoveOnly("one".into()))]);
        let change = segment.pop_front::<MoveOnly>().unwrap();
        let Change::Insert(_, value) = change else {
            panic!("unexpected removal");
        };
        let InputStorage::Owned(value) = value.storage else {
            panic!("unique value was borrowed");
        };
        assert_eq!(value.0, "one");
    }

    #[test]
    fn raw_segment_splits_owned_ranges_without_cloning_values() {
        let segment = Segment::new(vec![
            item(1, MoveOnly("one".into())),
            item(2, MoveOnly("two".into())),
            item(3, MoveOnly("three".into())),
        ]);
        let (mut first, tail) = segment.split_prefix(1);
        let (mut second, tail) = tail.unwrap().split_prefix(1);
        let mut third = tail.unwrap();

        let Change::Insert(_, first) = first.pop_front::<MoveOnly>().unwrap()
        else {
            panic!("unexpected removal");
        };
        let Change::Insert(_, second) = second.pop_front::<MoveOnly>().unwrap()
        else {
            panic!("unexpected removal");
        };
        let Change::Insert(_, third) = third.pop_front::<MoveOnly>().unwrap()
        else {
            panic!("unexpected removal");
        };

        let InputStorage::Owned(first) = first.storage else {
            panic!("owned range borrowed its value");
        };
        let InputStorage::Owned(second) = second.storage else {
            panic!("owned range borrowed its value");
        };
        let InputStorage::Owned(third) = third.storage else {
            panic!("owned range borrowed its value");
        };
        assert_eq!(first.0, "one");
        assert_eq!(second.0, "two");
        assert_eq!(third.0, "three");
    }

    #[test]
    fn raw_segment_split_ranges_drop_each_item_exactly_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let segment = Segment::new(vec![
            item(1, DropCount(Arc::clone(&drops))),
            item(2, DropCount(Arc::clone(&drops))),
            item(3, DropCount(Arc::clone(&drops))),
            item(4, DropCount(Arc::clone(&drops))),
        ]);
        let (mut first, tail) = segment.split_prefix(2);
        let (second, third) = tail.unwrap().split_prefix(1);

        let Change::Insert(_, moved) = first.pop_front::<DropCount>().unwrap()
        else {
            panic!("unexpected removal");
        };
        let InputStorage::Owned(moved) = moved.storage else {
            panic!("owned range borrowed its value");
        };
        drop(first);
        drop(second);
        drop(third);
        assert_eq!(drops.load(Ordering::Relaxed), 3);
        drop(moved);
        assert_eq!(drops.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn raw_segment_slices_shared_fanout_leases_independently() {
        let mut leases = Segment::new(vec![
            item(1, 10_u64),
            item(2, 20_u64),
            item(3, 30_u64),
        ])
        .fan_out(2);
        let (mut first, tail) =
            leases.next().expect("first lease").split_prefix(2);
        let mut tail = tail.unwrap();
        let mut whole = leases.next().expect("second lease");

        let Change::Insert(_, value) = first.pop_front::<u64>().unwrap() else {
            panic!("unexpected removal");
        };
        assert_eq!(*value.as_ref(), 10);
        let Change::Insert(_, value) = tail.pop_front::<u64>().unwrap() else {
            panic!("unexpected removal");
        };
        assert_eq!(*value.as_ref(), 30);
        let Change::Insert(_, value) = whole.pop_front::<u64>().unwrap() else {
            panic!("unexpected removal");
        };
        assert_eq!(*value.as_ref(), 10);
    }

    #[test]
    fn raw_segment_fan_out_clones_identities_and_borrows_values() {
        let segment = Segment::new(vec![item(1, MoveOnly("one".into()))]);
        for mut lease in segment.fan_out(2) {
            let change = lease.pop_front::<MoveOnly>().unwrap();
            let Change::Insert(key, value) = change else {
                panic!("unexpected removal");
            };
            assert_eq!(key, 1);
            let InputStorage::Borrowed(value) = value.storage else {
                panic!("shared value was moved");
            };
            assert_eq!(value.0, "one");
        }
    }

    #[test]
    fn raw_segment_final_lease_recovers_ownership() {
        let segment = Segment::new(vec![item(1, MoveOnly("one".into()))]);
        let mut leases = segment.fan_out(2);
        drop(leases.next().expect("first lease"));
        let mut lease = leases.next().expect("final lease");
        lease.promote_if_unique();
        let change = lease.pop_front::<MoveOnly>().unwrap();
        let Change::Insert(_, value) = change else {
            panic!("unexpected removal");
        };
        let InputStorage::Owned(value) = value.storage else {
            panic!("final lease remained borrowed");
        };
        assert_eq!(value.0, "one");
    }

    #[test]
    fn raw_segment_shared_cursors_are_independent() {
        let segment = Segment::new(vec![item(1, 10_u64), item(2, 20)]);
        for mut lease in segment.fan_out(2) {
            let Change::Insert(key, _) = lease.pop_front::<u64>().unwrap()
            else {
                panic!("unexpected removal");
            };
            assert_eq!(key, 1);
        }
    }

    #[test]
    fn raw_segment_shared_lease_can_fan_out_again() {
        let drops = Arc::new(AtomicUsize::new(0));
        let segment =
            Segment::new(vec![item(1, DropCount(Arc::clone(&drops)))]);
        let mut leases = segment.fan_out(2);
        let first = leases.next().expect("first lease");
        let second = leases.next().expect("second lease");

        let mut nested = first.fan_out(2);
        let nested_first = nested.next().expect("first nested lease");
        let nested_second = nested.next().expect("second nested lease");

        drop(second);
        drop(nested_first);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(nested_second);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn raw_segment_drops_unique_values_exactly_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let segment = Segment::new(vec![
            item(1, DropCount(Arc::clone(&drops))),
            item(2, DropCount(Arc::clone(&drops))),
        ]);
        drop(segment);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn raw_segment_drops_moved_and_unread_values_exactly_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut segment = Segment::new(vec![
            item(1, DropCount(Arc::clone(&drops))),
            item(2, DropCount(Arc::clone(&drops))),
        ]);
        let first = segment.pop_front::<DropCount>().unwrap();
        drop(first);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        drop(segment);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn raw_segment_shared_storage_drops_values_exactly_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let segment =
            Segment::new(vec![item(1, DropCount(Arc::clone(&drops)))]);
        let mut leases = segment.fan_out(3);
        let mut first = leases.next().expect("first lease");
        let _ = first.pop_front::<DropCount>();
        drop(first);
        drop(leases);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn raw_segment_zero_subscribers_drop_the_batch() {
        let drops = Arc::new(AtomicUsize::new(0));
        let segment =
            Segment::new(vec![item(1, DropCount(Arc::clone(&drops)))]);
        assert_eq!(segment.fan_out(0).len(), 0);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn raw_segment_rejected_partial_fan_out_drops_the_batch() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut segment = Segment::new(vec![
            item(1, DropCount(Arc::clone(&drops))),
            item(2, DropCount(Arc::clone(&drops))),
        ]);
        let first = segment.pop_front::<DropCount>().unwrap();
        drop(first);
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                segment.fan_out(2)
            }));
        assert!(result.is_err());
        assert_eq!(drops.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn raw_segment_drop_unwind_still_drops_the_remaining_range() {
        let drops = Arc::new(AtomicUsize::new(0));
        let segment = Segment::new(vec![
            item(
                1,
                PanicDrop {
                    drops: Arc::clone(&drops),
                    panic: true,
                },
            ),
            item(
                2,
                PanicDrop {
                    drops: Arc::clone(&drops),
                    panic: false,
                },
            ),
        ]);
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                drop(segment);
            }));
        assert!(result.is_err());
        assert_eq!(drops.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn raw_segment_supports_empty_and_zero_sized_batches() {
        let mut empty = Segment::<u64>::new::<()>(Vec::new());
        assert!(empty.pop_front::<()>().is_none());

        let mut zst = Segment::new(vec![item(1, ()), item(2, ())]);
        assert!(zst.pop_front::<()>().is_some());
        drop(zst);
    }
}
