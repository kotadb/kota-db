use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseJobPayload {
    pub git_url: String,
    pub provider: Option<String>,
    pub branch: Option<String>,
    pub requested_at: Option<DateTime<Utc>>,
    pub settings: Option<JsonValue>,
    pub event_type: Option<String>,
    pub delivery_id: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub commits: Option<JsonValue>,
    pub webhook_delivery_id: Option<i64>,
    pub changes: Option<JsonValue>,
}

impl SupabaseJobPayload {
    pub fn parse(payload: JsonValue) -> Self {
        serde_json::from_value(payload).unwrap_or_else(|_| SupabaseJobPayload {
            git_url: String::new(),
            provider: None,
            branch: None,
            requested_at: None,
            settings: None,
            event_type: None,
            delivery_id: None,
            git_ref: None,
            commits: None,
            webhook_delivery_id: None,
            changes: None,
        })
    }
}

pub fn merge_settings(base: &JsonValue, overrides: Option<&JsonValue>) -> JsonValue {
    let mut merged = base.clone();
    if let Some(overrides) = overrides {
        match (merged.as_object_mut(), overrides.as_object()) {
            (Some(base_obj), Some(override_obj)) => {
                for (key, value) in override_obj {
                    if key == "options" {
                        let merged_options = merge_options(base_obj.get("options"), value);
                        base_obj.insert(key.clone(), merged_options);
                    } else {
                        base_obj.insert(key.clone(), value.clone());
                    }
                }
            }
            _ => {
                merged = overrides.clone();
            }
        }
    }
    merged
}

fn merge_options(base: Option<&JsonValue>, overrides: &JsonValue) -> JsonValue {
    match (base.and_then(|v| v.as_object()), overrides.as_object()) {
        (Some(base_map), Some(override_map)) => {
            let mut merged = base_map.clone();
            for (key, value) in override_map {
                merged.insert(key.clone(), value.clone());
            }
            JsonValue::Object(merged)
        }
        (None, Some(map)) => JsonValue::Object(map.clone()),
        _ => overrides.clone(),
    }
}

pub fn options_map(settings: &JsonValue) -> Option<&JsonMap<String, JsonValue>> {
    settings.get("options").and_then(|v| v.as_object())
}

pub fn option_bool(settings: &JsonValue, key: &str) -> Option<bool> {
    options_map(settings).and_then(|opts| opts.get(key).and_then(|v| v.as_bool()))
}

pub fn option_usize(settings: &JsonValue, key: &str) -> Option<usize> {
    options_map(settings).and_then(|opts| {
        opts.get(key)
            .and_then(|v| v.as_u64())
            .and_then(|v| usize::try_from(v).ok())
    })
}
