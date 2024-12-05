use std::cmp::Ordering;

pub type Time = Option<(u8, u8)>;

#[derive(
    Clone, Debug, Default, PartialEq, Eq, Hash, derive_more::Deref, derive_more::IntoIterator,
)]
pub struct Schedule<V: Clone> {
    items: im::Vector<(Time, V)>,
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    derive_more::Deref,
    derive_more::DerefMut,
    derive_more::IntoIterator,
)]
pub struct ScheduleKv<K: Eq + Clone, V: Eq + Clone>(Schedule<(K, V)>);

impl<V> Schedule<V>
where
    V: Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            items: im::Vector::new(),
        }
    }

    pub fn insert(&mut self, t: Option<u8>, v: V) {
        if let Some(t) = t {
            let time = if let Some(((tt, b), _)) = self
                .items
                .iter()
                .filter_map(|(tb, v)| Some((tb.as_ref()?, v)))
                .skip_while(|&((tt, _), _)| *tt < t)
                .take_while(|&((tt, _), _)| *tt <= t)
                .last()
            {
                assert_eq!(t, *tt);
                (t, b + 1)
            } else {
                (t, 0)
            };
            self.items.push_back((Some(time), v));
        } else {
            if self.items.iter().find(|(_, vv)| v == *vv).is_none() {
                self.items.push_back((None, v));
            }
        }
        self.items.sort_by(|(tb0, _), (tb1, _)| match (tb0, tb1) {
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        });
    }

    pub fn pop(&mut self) -> Option<V> {
        // if the last item has a None time, don't pop
        self.items.front()?.0?;
        let (time, v) = self.items.pop_front()?;
        if let Some((t, b)) = time {
            assert_eq!(b, 0);
            for (time, _) in self.items.iter_mut() {
                if let Some((tt, bb)) = time {
                    if *tt == t {
                        *bb -= 1;
                    }
                    *tt -= t;
                }
            }
        }
        Some(v)
    }

    pub fn remove_by(&mut self, f: impl Fn(&V) -> bool) -> Option<V> {
        let i = self
            .items
            .iter()
            .enumerate()
            .find(|(_, (_, v))| f(v))
            .map(|(i, _)| i)?;
        Some(self.items.remove(i).1)
    }
}

impl<K: Eq + Clone, V: Eq + Clone> ScheduleKv<K, V> {
    pub fn remove_key(&mut self, k: &K) -> Option<V> {
        self.0.remove_by(|(kk, _)| kk == k).map(|(_, v)| v)
    }

    pub fn has_key(&self, k: &K) -> bool {
        self.0.items.iter().any(|(_, (kk, _))| kk == k)
    }

    pub fn get_key(&self, k: &K) -> Option<&V> {
        self.0
            .items
            .iter()
            .find(|(_, (kk, _))| kk == k)
            .map(|(_, (_, v))| v)
    }

    pub fn insert_kv(&mut self, t: Option<u8>, k: K, v: V) -> bool {
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
        schedule.insert(Some(2), "b");
        schedule.insert(Some(5), "w");
        schedule.insert(Some(2), "a");
        schedule.insert(None, "o");
        schedule.insert(Some(1), "z");
        schedule.insert(Some(1), "y");

        assert_eq!(schedule.remove_by(|v| *v == "a"), Some("a"));
        assert_eq!(schedule.remove_by(|v| *v == "o"), Some("o"));
    }

    #[test]
    fn test_insert_and_pop() {
        let mut schedule = Schedule::new();
        schedule.insert(Some(2), "b");
        schedule.insert(Some(5), "w");
        schedule.insert(Some(2), "a");
        schedule.insert(None, "1");
        schedule.insert(Some(0), "o");
        schedule.insert(Some(1), "z");
        schedule.insert(None, "0");
        schedule.insert(Some(1), "y");
        schedule.insert(None, "0");

        assert_eq!(
            schedule.items,
            im::vector![
                (Some((0, 0)), "o"),
                (Some((1, 0)), "z"),
                (Some((1, 1)), "y"),
                (Some((2, 0)), "b"),
                (Some((2, 1)), "a"),
                (Some((5, 0)), "w"),
                (None, "0"),
                (None, "1"),
            ]
        );

        assert_eq!(schedule.pop(), Some("o"));

        assert_eq!(
            schedule.items,
            im::vector![
                (Some((1, 0)), "z"),
                (Some((1, 1)), "y"),
                (Some((2, 0)), "b"),
                (Some((2, 1)), "a"),
                (Some((5, 0)), "w"),
                (None, "0"),
                (None, "1"),
            ]
        );

        assert_eq!(schedule.pop(), Some("z"));

        assert_eq!(
            schedule.items,
            im::vector![
                (Some((0, 0)), "y"),
                (Some((1, 0)), "b"),
                (Some((1, 1)), "a"),
                (Some((4, 0)), "w"),
                (None, "0"),
                (None, "1"),
            ]
        );

        assert_eq!(schedule.pop(), Some("y"));

        assert_eq!(
            schedule.items,
            im::vector![
                (Some((1, 0)), "b"),
                (Some((1, 1)), "a"),
                (Some((4, 0)), "w"),
                (None, "0"),
                (None, "1"),
            ]
        );

        assert_eq!(schedule.pop(), Some("b"));

        assert_eq!(
            schedule.items,
            im::vector![
                (Some((0, 0)), "a"),
                (Some((3, 0)), "w"),
                (None, "0"),
                (None, "1"),
            ]
        );

        assert_eq!(schedule.pop(), Some("a"));

        assert_eq!(
            schedule.items,
            im::vector![(Some((3, 0)), "w"), (None, "0"), (None, "1"),]
        );

        assert_eq!(schedule.pop(), Some("w"));

        assert_eq!(schedule.items, im::vector![(None, "0"), (None, "1"),]);

        assert_eq!(schedule.pop(), None);
    }
}
