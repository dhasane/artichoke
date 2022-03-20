use core::slice::{Iter, IterMut};

use super::TinyArray;

impl<'a, T> IntoIterator for &'a TinyArray<T>
where
    T: Default,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut TinyArray<T>
where
    T: Default,
{
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
