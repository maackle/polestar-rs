use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Schedule<V> {
    items: VecDeque<((u8, u8), V)>,
}

impl<V> Schedule<V> {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, (t, v): (u8, V)) {
        let b = if let Some(((_, b), _)) = self
            .items
            .iter()
            .skip_while(|((tt, _), _)| *tt < t)
            .take_while(|((tt, _), _)| *tt <= t)
            .last()
        {
            b + 1
        } else {
            0
        };
        self.items.push_back(((t, b), v));
        self.items.make_contiguous().sort_by_key(|(tb, _)| *tb);
    }

    pub fn pop(&mut self) -> Option<V> {
        let ((t, b), v) = self.items.pop_front()?;
        assert_eq!(b, 0);
        for ((tt, bb), _) in self.items.iter_mut() {
            if *tt == t {
                *bb -= 1;
            }
            *tt -= t;
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn test_schedule() {
        let mut schedule = Schedule::new();
        schedule.insert((2, "b"));
        schedule.insert((5, "w"));
        schedule.insert((2, "a"));
        schedule.insert((0, "o"));
        schedule.insert((1, "z"));
        schedule.insert((1, "y"));

        assert_eq!(
            schedule.items,
            vec![
                ((0, 0), "o"),
                ((1, 0), "z"),
                ((1, 1), "y"),
                ((2, 0), "b"),
                ((2, 1), "a"),
                ((5, 0), "w")
            ]
        );

        assert_eq!(schedule.pop(), Some("o"));

        assert_eq!(
            schedule.items,
            vec![
                ((1, 0), "z"),
                ((1, 1), "y"),
                ((2, 0), "b"),
                ((2, 1), "a"),
                ((5, 0), "w")
            ]
        );

        assert_eq!(schedule.pop(), Some("z"));

        assert_eq!(
            schedule.items,
            vec![((0, 0), "y"), ((1, 0), "b"), ((1, 1), "a"), ((4, 0), "w")]
        );

        assert_eq!(schedule.pop(), Some("y"));

        assert_eq!(
            schedule.items,
            vec![((1, 0), "b"), ((1, 1), "a"), ((4, 0), "w")]
        );

        assert_eq!(schedule.pop(), Some("b"));

        assert_eq!(schedule.items, vec![((0, 0), "a"), ((3, 0), "w")]);

        assert_eq!(schedule.pop(), Some("a"));

        assert_eq!(schedule.items, vec![((3, 0), "w")]);

        assert_eq!(schedule.pop(), Some("w"));
    }
}
