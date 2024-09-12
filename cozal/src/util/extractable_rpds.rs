use std::{
    borrow::Borrow,
    hash::{Hash, RandomState},
};

use archery::SharedPointerKind;

struct ExtractableHashTrieMap<K, V, P: SharedPointerKind>(rpds::HashTrieMap<K, V, P, RandomState>);

// note the additional requirements that K and V are clone. this differs from rpds::HashTrieMap,
// and allows us to assume Ks and Vs are unique unless they're clone.
impl<K, V, P: SharedPointerKind> Clone for ExtractableHashTrieMap<K, V, P>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K, V, P: SharedPointerKind> ExtractableHashTrieMap<K, V, P>
where
    K: Hash + Eq,
{
    #[must_use]
    pub fn new() -> Self {
        Self(rpds::HashTrieMap::new_with_hasher_and_ptr_kind(
            RandomState::default(),
        ))
    }

    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.0.get(key)
    }

    #[must_use]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.0.get_key_value(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.0.insert_mut(key, value)
    }

    #[must_use]
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
        V: Clone,
    {
        self.0.get(key).cloned()
    }
}
