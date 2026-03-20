pub(crate) fn normalize_filter_expression(filter: &str) -> Option<String> {
    let canonical = filter
        .trim()
        .replace(" and ", " && ")
        .replace(" AND ", " && ")
        .replace(" or ", " || ")
        .replace(" OR ", " || ");
    let canonical = canonical
        .replace('&', "&&")
        .replace("&&&&", "&&")
        .replace("||", " || ")
        .replace(" &&  && ", " && ");
    let canonical = canonical
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" && && ", " && ")
        .replace(" || || ", " || ");

    if canonical.is_empty() {
        return None;
    }

    if canonical
        .chars()
        .any(|character| "=!<>&|()".contains(character))
    {
        return Some(normalize_explicit(&canonical));
    }

    let tokens = canonical
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return None;
    }

    Some(
        tokens
            .into_iter()
            .map(shorthand_clause)
            .collect::<Vec<_>>()
            .join(" && "),
    )
}

fn normalize_explicit(filter: &str) -> String {
    let trimmed = filter.trim();
    if trimmed.starts_with("&&")
        || trimmed.starts_with("||")
        || trimmed.ends_with("&&")
        || trimmed.ends_with("||")
        || trimmed.contains("&& &&")
        || trimmed.contains("|| ||")
    {
        return trimmed.to_string();
    }

    let raw_clauses = filter
        .split("&&")
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .collect::<Vec<_>>();

    if raw_clauses.is_empty() {
        return filter.to_string();
    }

    raw_clauses
        .into_iter()
        .map(|clause| {
            if let Some((key, value)) = clause.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if key == "protocol" {
                    return format!("protocol={}", value.to_ascii_lowercase());
                }
                if key == "port" {
                    return format!("port={value}");
                }
                return clause.to_string();
            }

            if clause.chars().any(|character| "!<>|()".contains(character)) {
                return clause.to_string();
            }

            shorthand_clause(clause)
        })
        .collect::<Vec<_>>()
        .join(" && ")
}

fn shorthand_clause(token: &str) -> String {
    if token.parse::<u64>().is_ok() {
        return format!("port={token}");
    }

    format!("protocol={}", token.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::normalize_filter_expression;

    #[test]
    fn normalizes_shorthand_tokens_like_desktop_and_live() {
        assert_eq!(
            normalize_filter_expression("udp and 443"),
            Some("protocol=udp && port=443".to_string())
        );
        assert_eq!(
            normalize_filter_expression("tcp & 80"),
            Some("protocol=tcp && port=80".to_string())
        );
    }

    #[test]
    fn normalizes_explicit_protocol_value_to_lowercase() {
        assert_eq!(
            normalize_filter_expression("protocol=HTTP && port=443"),
            Some("protocol=http && port=443".to_string())
        );
    }

    #[test]
    fn returns_none_for_empty_expression() {
        assert_eq!(normalize_filter_expression("   "), None);
    }
}
