//! A check `serde_json::Value` cannot do: it keeps only the last of two equal
//! keys in one object, without a word.

use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// Walks the whole value and fails on the first object that holds a key twice.
struct Walk;

struct WalkVisitor;

impl<'de> Visitor<'de> for WalkVisitor {
    type Value = Walk;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Walk, E> {
        Ok(Walk)
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Walk, E> {
        Ok(Walk)
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Walk, E> {
        Ok(Walk)
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Walk, E> {
        Ok(Walk)
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<Walk, E> {
        Ok(Walk)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Walk, E> {
        Ok(Walk)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Walk, A::Error> {
        while seq.next_element::<Walk>()?.is_some() {}
        Ok(Walk)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Walk, A::Error> {
        let mut seen = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(key));
            }
            map.next_value::<Walk>()?;
        }
        Ok(Walk)
    }
}

impl<'de> Deserialize<'de> for Walk {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Walk, D::Error> {
        d.deserialize_any(WalkVisitor)
    }
}

/// The first key that appears twice in one object of `text`, if any.
///
/// `text` must already be valid JSON; anything else gives `None`.
pub fn repeated_key(text: &str) -> Option<String> {
    let mut de = serde_json::Deserializer::from_str(text);
    match Walk::deserialize(&mut de) {
        Ok(_) => None,
        Err(e) => {
            // `custom` carries the key, followed by serde_json's " at line .. column ..".
            let message = e.to_string();
            let end = message.rfind(" at line ").unwrap_or(message.len());
            Some(message[..end].to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_repeated_key_at_any_depth() {
        assert_eq!(repeated_key(r#"{"a":1,"a":2}"#).as_deref(), Some("a"));
        assert_eq!(
            repeated_key(r#"[{"x":1},{"b":{"c":1,"c":1}}]"#).as_deref(),
            Some("c")
        );
    }

    #[test]
    fn same_key_in_different_objects_is_not_repeated() {
        assert_eq!(repeated_key(r#"[{"a":1},{"a":2}]"#), None);
        assert_eq!(repeated_key(r#"{"a":{"a":1}}"#), None);
        assert_eq!(repeated_key("[1,null,true,\"x\",1.5]"), None);
    }
}
