use crate::SkipList;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl<T: Serialize + Clone + PartialOrd> Serialize for SkipList<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let eles: Vec<_> = self.iter_all().collect();
        eles.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de> + PartialOrd + Clone> Deserialize<'de> for SkipList<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let eles: Vec<T> = Deserialize::deserialize(deserializer)?;
        Ok(SkipList::from(eles))
    }
}

#[cfg(test)]
mod test_serde {
    use crate::SkipList;
    use serde_json;
    #[test]
    fn test_serde() {
        let mut s = SkipList::new();
        for i in 0..10u32 {
            s.insert(i);
        }
        let ser = serde_json::to_string(&s).expect("Failed to serialize!");
        let back = serde_json::from_str(&ser).expect("Failed to deserialize!");
        assert_eq!(s, back);
    }
}
