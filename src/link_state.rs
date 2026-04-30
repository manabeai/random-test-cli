use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum LinkStateError {
    #[error("input does not contain a state value")]
    MissingState,
    #[error("invalid URL encoding: {0}")]
    UrlDecode(String),
}

pub fn decode_input(input: &str) -> Result<String, LinkStateError> {
    let state = extract_state(input)?;
    decode_state(&state)
}

pub fn extract_state(input: &str) -> Result<String, LinkStateError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(LinkStateError::MissingState);
    }

    if let Ok(url) = Url::parse(trimmed) {
        if let Some((_, value)) = url.query_pairs().find(|(key, _)| key == "state") {
            return Ok(value.into_owned());
        }
        return Err(LinkStateError::MissingState);
    }

    if let Some(query_start) = trimmed.find('?') {
        let query = &trimmed[query_start + 1..];
        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key == "state" {
                return Ok(value.to_owned());
            }
        }
    }

    Ok(trimmed.to_owned())
}

pub fn decode_state(state: &str) -> Result<String, LinkStateError> {
    let decoded =
        urlencoding::decode(state).map_err(|err| LinkStateError::UrlDecode(err.to_string()))?;
    Ok(decoded.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_state_from_full_url() {
        let input = "https://manabeai.github.io/cp-ast-ecosystems/?state=%7B%22schema_version%22%3A1%7D";
        assert_eq!(
            extract_state(input).unwrap(),
            r#"{"schema_version":1}"#
        );
    }

    #[test]
    fn accepts_state_value_directly() {
        assert_eq!(extract_state("abc").unwrap(), "abc");
    }

    #[test]
    fn decodes_url_encoded_json_state() {
        let json = r#"{"schema_version":1}"#;
        let state = urlencoding::encode(json);
        assert_eq!(decode_state(&state).unwrap(), json);
    }
}
