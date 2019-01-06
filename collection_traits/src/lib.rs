use std::borrow::Borrow;

trait Set<T> {
    fn contains<Q>(&self, item: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a T> + 'a>;
    fn insert(&mut self, item: T);
    fn remove<Q>(&mut self, item: &Q)
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized;
}

trait Map<K, V> {
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;

    fn insert(&mut self, key: K, val: V);
    fn remove<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;
}

trait MultiMap<K, V> {
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;
    fn get<'a, Q>(&'a self, key: &Q) -> Box<dyn Iterator<Item = &V> + 'a>;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;

    fn insert(&mut self, key: K, val: V);
    fn remove<Q, R>(&mut self, key: &Q, val: &R)
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Borrow<R>,
        R: Ord + ?Sized;
    fn remove_all<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized;
}

