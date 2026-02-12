use serde_json::Value;

const MASKED: &str = "***";

pub fn mask_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return MASKED.to_string();
    }

    let mut chars = trimmed.chars();
    let first = chars.next().unwrap_or('*');
    let remaining = chars.count();
    format!("{first}{}", "*".repeat(remaining.max(2)))
}

pub fn mask_email(email: &str) -> String {
    let trimmed = email.trim();
    let Some((local, domain)) = trimmed.split_once('@') else {
        return MASKED.to_string();
    };

    let local_first = local.chars().next().unwrap_or('*');
    let mut domain_parts = domain.split('.');
    let domain_label = domain_parts.next().unwrap_or_default();
    let tld = domain_parts.collect::<Vec<_>>().join(".");
    let domain_first = domain_label.chars().next().unwrap_or('*');

    if tld.is_empty() {
        format!("{local_first}***@{domain_first}***")
    } else {
        format!("{local_first}***@{domain_first}***.{tld}")
    }
}

pub fn mask_ip(ip: &str) -> String {
    if ip.contains(':') {
        let mut parts = ip.split(':').collect::<Vec<_>>();
        if parts.len() > 4 {
            parts.truncate(4);
        }
        return format!("{}::/64", parts.join(":"));
    }

    let mut parts = ip.split('.').collect::<Vec<_>>();
    if parts.len() == 4 {
        parts[3] = "0";
        return format!("{}.0/24", parts[..3].join("."));
    }

    MASKED.to_string()
}

pub fn mask_user_agent(user_agent: &str) -> String {
    if user_agent.is_empty() {
        return MASKED.to_string();
    }
    let visible: String = user_agent.chars().take(12).collect();
    format!("{visible}***")
}

pub fn mask_pii_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let masked = map
                .iter()
                .map(|(key, val)| {
                    let lowered = key.to_ascii_lowercase();
                    let out = if is_pii_key(&lowered) {
                        mask_json_scalar(&lowered, val)
                    } else {
                        mask_pii_json(val)
                    };
                    (key.clone(), out)
                })
                .collect();
            Value::Object(masked)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(mask_pii_json).collect()),
        _ => value.clone(),
    }
}

fn is_pii_key(key: &str) -> bool {
    [
        "email",
        "full_name",
        "name",
        "secret",
        "token",
        "mfa",
        "ip",
        "user_agent",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn mask_json_scalar(key: &str, value: &Value) -> Value {
    if let Some(raw) = value.as_str() {
        if key.contains("email") {
            return Value::String(mask_email(raw));
        }
        if key.contains("name") {
            return Value::String(mask_name(raw));
        }
        if key.contains("ip") {
            return Value::String(mask_ip(raw));
        }
        if key.contains("user_agent") {
            return Value::String(mask_user_agent(raw));
        }
    }
    Value::String(MASKED.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mask_name_keeps_first_character() {
        assert_eq!(mask_name("Alice"), "A****");
    }

    #[test]
    fn mask_email_keeps_shape() {
        assert_eq!(mask_email("alice@example.com"), "a***@e***.com");
    }

    #[test]
    fn mask_pii_json_masks_known_keys_recursively() {
        let input = json!({
            "email": "alice@example.com",
            "nested": {
                "full_name": "Alice Example",
                "details": "ok"
            }
        });
        let masked = mask_pii_json(&input);
        assert_eq!(masked["email"], "a***@e***.com");
        assert_eq!(masked["nested"]["full_name"], "A************");
        assert_eq!(masked["nested"]["details"], "ok");
    }
}
