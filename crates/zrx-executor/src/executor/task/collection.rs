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

//! Task collection.

use std::vec::IntoIter;

use super::Task;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Task collection.
///
/// This data type represents a collection of tasks that can either be consumed
/// through iteration, or executed recursively via [`Tasks::execute`]. Anything
/// returned by [`Task::execute`] must be convertible into [`Tasks`], including
/// another task, multiple tasks, and the unit value.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_executor::task::Tasks;
/// use zrx_executor::Executor;
///
/// // Create executor and submit task
/// let executor = Executor::default();
/// executor.submit(|| {
///     println!("Task 1");
///
///     // Create subtasks
///     let mut tasks = Tasks::new();
///     tasks.add(|| println!("Task 1.1"));
///     tasks.add(|| println!("Task 1.2"));
///     tasks.add(|| println!("Task 1.3"));
///     tasks
/// })?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct Tasks {
    /// Vector of tasks.
    inner: Vec<Box<dyn Task>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Tasks {
    /// Creates a task collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection
    /// let tasks = Tasks::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a task to the task collection.
    ///
    /// This method adds a [`Task`] to the collection, which can then either be
    /// consumed via [`Tasks::into_iter`] or executed via [`Tasks::execute`],
    /// depending on the specifics of the execution strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection and add tasks
    /// let mut tasks = Tasks::new();
    /// tasks.add(|| println!("Task 1"));
    /// tasks.add(|| println!("Task 2"));
    /// tasks.add(|| println!("Task 3"));
    /// ```
    #[inline]
    pub fn add<T>(&mut self, task: T) -> &mut Self
    where
        T: Task,
    {
        self.inner.push(Box::new(task));
        self
    }

    /// Executes all tasks in the task collection.
    ///
    /// This method executes all tasks recursively in depth-first order, so if
    /// a task returns further subtasks, they are executed before all others.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection and add tasks
    /// let mut tasks = Tasks::new();
    /// tasks.add(|| println!("Task 1"));
    /// tasks.add(|| println!("Task 2"));
    /// tasks.add(|| println!("Task 3"));
    ///
    /// // Execute task collection
    /// tasks.execute();
    /// ```
    pub fn execute(mut self) {
        // Since we're using the inner vector as a stack, we need to reverse it
        // to ensure that the first task added is the first one executed
        self.inner.reverse();
        while let Some(task) = self.inner.pop() {
            // Execute the current task, and if it returns further subtasks,
            // push them onto the stack in reverse order
            self.inner.extend(task.execute().into_iter().rev());
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl Tasks {
    /// Returns the number of tasks.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any tasks.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<()> for Tasks {
    /// Creates a task collection from the unit value.
    ///
    /// This implementation makes the API more flexible, as it allows to just
    /// return nothing from a task, which is probably the common case.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection from unit value
    /// let tasks = Tasks::from(());
    /// assert!(tasks.is_empty());
    /// ```
    #[inline]
    fn from((): ()) -> Self {
        Self::default()
    }
}

impl<T> From<T> for Tasks
where
    T: Task,
{
    /// Creates a task collection from a task.
    ///
    /// This implementation creates a task collection from a single task, which
    /// allows to conveniently return a single closure from a task.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection from task
    /// let tasks = Tasks::from(|| println!("Task"));
    /// assert_eq!(tasks.len(), 1);
    /// ```
    #[inline]
    fn from(task: T) -> Self {
        Self::from_iter([task])
    }
}

// ----------------------------------------------------------------------------

impl<I> FromIterator<I> for Tasks
where
    I: Task,
{
    /// Creates a task collection from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection from iterator
    /// let tasks = Tasks::from_iter([
    ///     || println!("Task 1"),
    ///     || println!("Task 2"),
    ///     || println!("Task 3"),
    /// ]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        let mut tasks = Self::new();
        for task in iter {
            tasks.add(task);
        }
        tasks
    }
}

impl IntoIterator for Tasks {
    type Item = Box<dyn Task>;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates an iterator over the task collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::task::Tasks;
    ///
    /// // Create task collection and add tasks
    /// let mut tasks = Tasks::new();
    /// tasks.add(|| println!("Task 1"));
    /// tasks.add(|| println!("Task 2"));
    /// tasks.add(|| println!("Task 3"));
    ///
    /// // Create iterator over tasks
    /// for task in tasks {
    ///     task.execute();
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
