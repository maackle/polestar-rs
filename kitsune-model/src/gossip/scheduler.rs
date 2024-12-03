use std::collections::VecDeque;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::Deref)]
pub struct Schedule<V> {
    items: VecDeque<((u8, u8), V)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::Deref, derive_more::DerefMut)]
pub struct ScheduleKv<K, V>(Schedule<(K, V)>);

impl<V> Schedule<V> {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, t: u8, v: V) {
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

    pub fn remove_by(&mut self, f: impl Fn(&V) -> bool) -> Option<((u8, u8), V)> {
        let i = self
            .items
            .iter()
            .enumerate()
            .find(|(_, (_, v))| f(v))
            .map(|(i, _)| i)?;
        self.items.remove(i)
    }
}

impl<K: Eq, V> ScheduleKv<K, V> {
    pub fn remove_key(&mut self, k: &K) -> Option<((u8, u8), (K, V))> {
        self.0.remove_by(|(kk, _)| kk == k)
    }

    pub fn has_key(&self, k: &K) -> bool {
        self.0.items.iter().any(|((_, _), (kk, _))| kk == k)
    }

    pub fn insert_kv(&mut self, t: u8, k: K, v: V) -> bool {
        if self.has_key(&k) {
            false
        } else {
            self.0.insert(t, (k, v));
            true
        }
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn test_remove() {
        let mut schedule = Schedule::new();
        schedule.insert(2, "b");
        schedule.insert(5, "w");
        schedule.insert(2, "a");
        schedule.insert(0, "o");
        schedule.insert(1, "z");
        schedule.insert(1, "y");

        assert_eq!(schedule.remove_by(|v| *v == "a"), Some(((2, 1), "a")));
    }

    #[test]
    fn test_insert_and_pop() {
        let mut schedule = Schedule::new();
        schedule.insert(2, "b");
        schedule.insert(5, "w");
        schedule.insert(2, "a");
        schedule.insert(0, "o");
        schedule.insert(1, "z");
        schedule.insert(1, "y");

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
