use std::cmp::Ordering;

/// Алгоритм симметрической разницы `A∆B` двух массивов.
///
/// Возвращается кортеж из двух массивов. В первом массиве лежит разница `A\B`,
/// во втором разница `B\A`.
pub fn symmetric_diff<A, B, E>(a: A, b: B) -> (Vec<E>, Vec<E>)
where
    A: IntoIterator<Item = E>,
    B: IntoIterator<Item = E>,
    E: Ord,
{
    let mut a = a.into_iter();
    let mut b = b.into_iter();
    let mut a_value = a.next();
    let mut b_value = b.next();
    let mut first = vec![];
    let mut second = vec![];

    while a_value.is_some() && b_value.is_some() {
        match a_value.as_ref().unwrap().cmp(&b_value.as_ref().unwrap()) {
            Ordering::Less => {
                first.push(a_value.unwrap());
                a_value = a.next();
            }
            Ordering::Greater => {
                second.push(b_value.unwrap());
                b_value = b.next();
            }
            Ordering::Equal => {
                a_value = a.next();
                b_value = b.next();
            }
        }
    }

    if let Some(value) = a_value {
        first.push(value);
        first.extend(a);
    }

    if let Some(value) = b_value {
        second.push(value);
        second.extend(b);
    }

    (first, second)
}

#[cfg(test)]
mod tests {
    mod symmetric_diff {
        use crate::tools::symmetric_diff;

        #[test]
        fn with_two_empty_arrays() {
            let a = vec![0; 0];
            let b = vec![0; 0];

            let (first, second) = symmetric_diff(a, b);
            assert!(first.is_empty());
            assert!(second.is_empty());
        }

        #[test]
        fn with_two_equal_arrays() {
            let a = vec![0, 1, 2];
            let b = vec![0, 1, 2];

            let (first, second) = symmetric_diff(a, b);
            assert!(first.is_empty());
            assert!(second.is_empty());
        }

        #[test]
        fn with_first_empty_array() {
            let a = vec![0; 0];
            let b = vec![0, 1];

            let (first, second) = symmetric_diff(a, b);
            assert!(first.is_empty());
            assert_eq!(vec![0, 1], second);
        }

        #[test]
        fn with_second_empty_array() {
            let a = vec![0, 1];
            let b = vec![0; 0];

            let (first, second) = symmetric_diff(a, b);
            assert_eq!(vec![0, 1], first);
            assert!(second.is_empty());
        }

        #[test]
        fn with_two_intersecting_arrays() {
            let a = vec![0, 1, 2];
            let b = vec![2, 3, 4];

            let (first, second) = symmetric_diff(a, b);
            assert_eq!(vec![0, 1], first);
            assert_eq!(vec![3, 4], second);
        }

        #[test]
        fn with_two_non_intersecting_arrays() {
            let a = vec![0, 1, 2];
            let b = vec![3, 4, 5];

            let (first, second) = symmetric_diff(a, b);
            assert_eq!(vec![0, 1, 2], first);
            assert_eq!(vec![3, 4, 5], second);
        }

        #[test]
        fn with_two_arrays_with_domains() {
            let a = vec!["example.org".to_string(), "yandex.ru".to_string()];
            let b = vec!["google.com".to_string(), "yandex.ru".to_string()];

            let (first, second) = symmetric_diff(a, b);
            assert_eq!(1, first.len());
            assert_eq!("example.org", first[0]);
            assert_eq!(1, second.len());
            assert_eq!("google.com", second[0]);
        }
    }
}
