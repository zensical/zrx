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

//! Bounded in-order release for monotonically sequenced values.

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

pub struct OrderedWindow<T> {
    slots: Option<Box<[Slot<T>]>>,
    capacity: usize,
    next: u64,
    pending: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> OrderedWindow<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity != 0, "ordered window capacity must be non-zero");
        Self {
            slots: None,
            capacity,
            next: 0,
            pending: 0,
        }
    }

    pub const fn next(&self) -> u64 {
        self.next
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn is_pending_empty(&self) -> bool {
        self.pending == 0
    }

    pub fn insert(&mut self, order: u64, value: T) -> Option<T> {
        assert!(order >= self.next, "ordered value inserted more than once");
        if order == self.next {
            self.advance();
            return Some(value);
        }

        let distance = order - self.next;
        assert!(
            distance < self.capacity as u64,
            "ordered value exceeds its bounded window"
        );
        let index = self.index(order);
        let slots = self.slots.get_or_insert_with(|| {
            std::iter::repeat_with(|| None)
                .take(self.capacity)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        });
        assert!(
            slots[index].is_none(),
            "ordered value inserted more than once"
        );
        slots[index] = Some((order, value));
        self.pending += 1;
        None
    }

    pub fn pop_ready(&mut self) -> Option<T> {
        let index = self.index(self.next);
        let slots = self.slots.as_mut()?;
        let (order, _) = slots[index].as_ref()?;
        assert_eq!(*order, self.next, "ordered window slot collided");
        let (_, value) =
            slots[index].take().expect("validated ordered window slot");
        self.pending -= 1;
        self.advance();
        Some(value)
    }

    pub fn pending_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots
            .iter_mut()
            .flat_map(|slots| slots.iter_mut())
            .filter_map(Option::as_mut)
            .map(|(_, value)| value)
    }

    fn advance(&mut self) {
        self.next = self
            .next
            .checked_add(1)
            .expect("ordered sequence overflowed");
    }

    fn index(&self, order: u64) -> usize {
        let capacity = u64::try_from(self.capacity)
            .expect("ordered window capacity exceeds its sequence space");
        usize::try_from(order % capacity)
            .expect("ordered window index exceeds addressable memory")
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type Slot<T> = Option<(u64, T)>;

#[cfg(test)]
mod tests {
    use super::OrderedWindow;

    fn permutations(
        values: &mut [u64], index: usize, check: &mut impl FnMut(&[u64]),
    ) {
        if index == values.len() {
            check(values);
            return;
        }

        for current in index..values.len() {
            values.swap(index, current);
            permutations(values, index + 1, check);
            values.swap(index, current);
        }
    }

    #[test]
    fn releases_the_contiguous_prefix_without_buffering_the_first_value() {
        let mut window = OrderedWindow::new(3);

        assert_eq!(window.insert(0, 'a'), Some('a'));
        assert!(window.is_pending_empty());
        assert_eq!(window.next(), 1);
    }

    #[test]
    fn repairs_bounded_out_of_order_completion() {
        let mut window = OrderedWindow::new(3);

        assert_eq!(window.insert(2, 'c'), None);
        assert_eq!(window.insert(1, 'b'), None);
        assert_eq!(window.insert(0, 'a'), Some('a'));
        assert_eq!(window.pop_ready(), Some('b'));
        assert_eq!(window.pop_ready(), Some('c'));
        assert_eq!(window.pop_ready(), None);
        assert!(window.is_pending_empty());
        assert_eq!(window.next(), 3);
    }

    #[test]
    fn reuses_ring_slots_after_the_sequence_wraps_its_capacity() {
        let mut window = OrderedWindow::new(3);

        assert_eq!(window.insert(2, 'c'), None);
        assert_eq!(window.insert(0, 'a'), Some('a'));
        assert_eq!(window.insert(3, 'd'), None);
        assert_eq!(window.insert(1, 'b'), Some('b'));
        assert_eq!(window.pop_ready(), Some('c'));
        assert_eq!(window.pop_ready(), Some('d'));
    }

    #[test]
    fn every_bounded_completion_order_releases_once_in_sequence() {
        for width in 1..=7 {
            let width = u64::try_from(width).expect("bounded test width");
            let mut order = (0..width).collect::<Vec<_>>();
            permutations(&mut order, 0, &mut |order| {
                let mut window = OrderedWindow::new(
                    usize::try_from(width).expect("bounded test width"),
                );
                let mut released = Vec::new();

                for &sequence in order {
                    if let Some(value) = window.insert(sequence, sequence) {
                        released.push(value);
                        while let Some(value) = window.pop_ready() {
                            released.push(value);
                        }
                    }
                }

                assert_eq!(released, (0..width).collect::<Vec<_>>());
                assert!(window.is_pending_empty());
                assert_eq!(window.next(), width);
            });
        }
    }

    #[test]
    #[should_panic(expected = "ordered value inserted more than once")]
    fn rejects_a_stale_sequence() {
        let mut window = OrderedWindow::new(2);
        assert_eq!(window.insert(0, 'a'), Some('a'));
        let _ = window.insert(0, 'a');
    }

    #[test]
    #[should_panic(expected = "ordered value inserted more than once")]
    fn rejects_a_duplicate_pending_sequence() {
        let mut window = OrderedWindow::new(2);
        assert_eq!(window.insert(1, 'b'), None);
        let _ = window.insert(1, 'b');
    }

    #[test]
    #[should_panic(expected = "ordered value exceeds its bounded window")]
    fn rejects_a_sequence_outside_the_window() {
        let mut window = OrderedWindow::new(2);
        let _ = window.insert(2, 'c');
    }
}
