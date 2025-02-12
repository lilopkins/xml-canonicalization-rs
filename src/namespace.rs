use std::{collections::HashMap, hash::Hash};

#[derive(Debug)]
pub struct Namespace {
    pub url: String,
}

#[derive(Debug)]
pub struct DepthSensitiveMap<K, V> {
    levels: Vec<HashMap<K, V>>,
}

impl<K, V> DepthSensitiveMap<K, V> {
    pub fn new() -> Self {
        Self { levels: vec![] }
    }

    pub fn remove_depth(&mut self, depth: usize) {
        while self.levels.len() > depth {
            self.levels.remove(depth);
        }
    }
}

impl<K, V> DepthSensitiveMap<K, V>
where
    K: Eq + Hash,
{
    pub fn insert_at_depth<IntoK: Into<K>>(
        &mut self,
        depth: usize,
        key: IntoK,
        value: V,
    ) -> Option<V> {
        while self.levels.len() <= depth {
            self.levels.push(HashMap::new());
        }
        self.levels[depth].insert(key.into(), value)
    }

    pub fn to_map(&self) -> HashMap<&K, &V> {
        let mut total_map = HashMap::new();

        for level in &self.levels {
            for (k, v) in level {
                total_map.insert(k, v);
            }
        }

        total_map
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[test]
    fn depth_map() {
        let mut map: DepthSensitiveMap<&str, &str> = DepthSensitiveMap::new();
        tracing::debug!("Initially: {map:?}");

        map.insert_at_depth(0, "TestA", "valueA");
        map.insert_at_depth(0, "TestB", "valueB");

        map.insert_at_depth(1, "TestA", "deeperA");

        let r = map.to_map();
        tracing::debug!("After setup: {map:?} (looks like: {r:?})");

        assert_eq!(r.get(&"TestA"), Some(&&"deeperA"));
        assert_eq!(r.get(&"TestB"), Some(&&"valueB"));

        map.remove_depth(1);
        let r = map.to_map();
        tracing::debug!("After dropping depth: {map:?} (looks like: {r:?})");

        assert_eq!(r.get(&"TestA"), Some(&&"valueA"));
        assert_eq!(r.get(&"TestB"), Some(&&"valueB"));
    }
}
