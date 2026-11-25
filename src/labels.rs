//! Label columns in the formats an export may use.

use std::collections::BTreeSet;

use crate::error::Fault;

/// Parses a label field into `(key, value)` pairs.
///
/// Accepted:
/// * BigQuery JSON: `[{"key":"env","value":"prod"}]`
/// * JSON object: `{"env":"prod"}`
/// * plain pairs: `env=prod;team=web` or `env:prod,team:web`
///
/// An empty field, `[]`, `{}` and `null` mean "no labels". A key that appears
/// twice is ambiguous and refused, whatever its values and in whatever format.
pub fn parse_labels(text: &str) -> Result<Vec<(String, String)>, Fault> {
    let labels = parse_pairs(text)?;
    let mut seen = BTreeSet::new();
    for (key, _) in &labels {
        if !seen.insert(key.as_str()) {
            return Err(Fault::Ambiguous(format!(
                "the label key {key:?} appears more than once"
            )));
        }
    }
    Ok(labels)
}

fn parse_pairs(text: &str) -> Result<Vec<(String, String)>, Fault> {
    let t = text.trim();
    if t.is_empty() || t == "null" {
        return Ok(Vec::new());
    }
    if t.starts_with('[') || t.starts_with('{') {
        return parse_json(t);
    }
    let mut out = Vec::new();
    for part in t.split([';', ',']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let split_at = part.find('=').or_else(|| part.find(':'));
        let Some(i) = split_at else {
            return Err(format!("label {part:?} has no '=' or ':'").into());
        };
        let key = part[..i].trim();
        if key.is_empty() {
            return Err(format!("label {part:?} has an empty key").into());
        }
        out.push((key.to_string(), part[i + 1..].trim().to_string()));
    }
    Ok(out)
}

fn json_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn parse_json(t: &str) -> Result<Vec<(String, String)>, Fault> {
    let value: serde_json::Value =
        serde_json::from_str(t).map_err(|e| format!("labels are not valid JSON: {e}"))?;
    if let Some(key) = crate::json::repeated_key(t) {
        return Err(Fault::Ambiguous(format!(
            "the JSON key {key:?} appears twice in one object"
        )));
    }
    let mut out = Vec::new();
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                let key = item.get("key").map(json_text).unwrap_or_default();
                if key.is_empty() {
                    return Err("a label in the JSON array has no \"key\"".into());
                }
                out.push((key, item.get("value").map(json_text).unwrap_or_default()));
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if k.is_empty() {
                    return Err("a label in the JSON object has an empty key".into());
                }
                out.push((k, json_text(&v)));
            }
        }
        _ => return Err("labels JSON must be an array or an object".into()),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[test]
    fn empty_forms_mean_no_labels() {
        for t in ["", "  ", "[]", "{}", "null"] {
            assert_eq!(parse_labels(t).unwrap(), vec![], "{t:?}");
        }
    }

    #[test]
    fn bigquery_json_array() {
        let got =
            parse_labels(r#"[{"key":"env","value":"prod"},{"key":"team","value":"web"}]"#).unwrap();
        assert_eq!(got, vec![kv("env", "prod"), kv("team", "web")]);
    }

    #[test]
    fn json_array_missing_value_is_empty_string() {
        assert_eq!(
            parse_labels(r#"[{"key":"env"}]"#).unwrap(),
            vec![kv("env", "")]
        );
        assert_eq!(
            parse_labels(r#"[{"key":"env","value":null}]"#).unwrap(),
            vec![kv("env", "")]
        );
    }

    #[test]
    fn json_array_without_key_is_an_error() {
        assert!(parse_labels(r#"[{"value":"x"}]"#).is_err());
    }

    #[test]
    fn json_object() {
        let got = parse_labels(r#"{"env":"prod","n":3}"#).unwrap();
        assert_eq!(got, vec![kv("env", "prod"), kv("n", "3")]);
    }

    #[test]
    fn invalid_json_is_an_error() {
        let e = parse_labels("[{oops").unwrap_err();
        assert!(matches!(e, Fault::Invalid(_)));
        assert!(e.to_string().contains("not valid JSON"), "{e}");
    }

    #[test]
    fn plain_pairs_with_equals_and_semicolon() {
        assert_eq!(
            parse_labels("env=prod;team=web").unwrap(),
            vec![kv("env", "prod"), kv("team", "web")]
        );
    }

    #[test]
    fn plain_pairs_with_colon_and_comma() {
        assert_eq!(
            parse_labels("env:prod, team:web").unwrap(),
            vec![kv("env", "prod"), kv("team", "web")]
        );
    }

    #[test]
    fn equals_wins_over_colon_and_value_may_contain_colon() {
        assert_eq!(
            parse_labels("url=http://x").unwrap(),
            vec![kv("url", "http://x")]
        );
    }

    #[test]
    fn pair_without_separator_is_an_error() {
        assert!(parse_labels("justakey").is_err());
        assert!(parse_labels("=value").is_err());
    }

    #[test]
    fn empty_segments_are_skipped() {
        assert_eq!(
            parse_labels("a=1;;b=2;").unwrap(),
            vec![kv("a", "1"), kv("b", "2")]
        );
    }

    #[test]
    fn a_repeated_label_key_is_ambiguous_in_every_format() {
        for t in [
            r#"{"env":"prod","env":"dev"}"#,
            r#"[{"key":"env","value":"prod"},{"key":"env","value":"dev"}]"#,
            "env=prod;env=dev",
            "env:prod,env=prod",
        ] {
            let e = parse_labels(t).unwrap_err();
            assert!(matches!(e, Fault::Ambiguous(_)), "{t}: {e:?}");
            assert!(e.to_string().contains("\"env\""), "{e}");
        }
    }

    #[test]
    fn a_repeated_key_inside_a_label_item_is_ambiguous() {
        let e = parse_labels(r#"[{"key":"a","key":"b","value":"x"}]"#).unwrap_err();
        assert!(matches!(e, Fault::Ambiguous(_)), "{e:?}");
    }

    #[test]
    fn the_same_key_in_different_objects_is_fine() {
        let got = parse_labels(r#"{"env":"a","nested":{"env":"b"}}"#).unwrap();
        assert_eq!(got.len(), 2);
    }
}
