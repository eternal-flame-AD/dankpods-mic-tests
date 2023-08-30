pub fn iter_continuous_range<T, I, FC>(iter: I, is_continuous: FC) -> impl Iterator<Item = (T, T)>
where
    T: Clone,
    I: Iterator<Item = T>,
    FC: Fn(&T, &T) -> bool,
{
    let mut iter = iter.peekable();
    std::iter::from_fn(move || {
        let first = iter.next()?;
        let mut last = first.clone();
        while let Some(next) = iter.peek() {
            if is_continuous(&last, next) {
                last = iter.next().unwrap();
            } else {
                break;
            }
        }
        Some((first, last))
    })
}

mod tests {
    use super::*;

    #[test]
    fn test_iter_continuous_range() {
        let v = vec![1, 2, 3, 5, 6, 7, 9, 10, 11];
        let mut iter = iter_continuous_range(v.iter(), |a, b| **a + 1 == **b);
        assert_eq!(iter.next(), Some((&1, &3)));
        assert_eq!(iter.next(), Some((&5, &7)));
        assert_eq!(iter.next(), Some((&9, &11)));
        assert_eq!(iter.next(), None);

        let v = Vec::<i32>::new();
        let mut iter = iter_continuous_range(v.iter(), |a, b| **a + 1 == **b);
        assert_eq!(iter.next(), None);
    }
}
