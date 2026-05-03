use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum LinkStateError {
    #[error("input does not contain a state value")]
    MissingState,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_state_from_full_url() {
        let input = "https://manabeai.github.io/cp-ast-ecosystems/?state=H4sIAAAAAAAA_6tWKi5JLCktVrIy1FFQyslPzk7JTM7WAwA_JkA_FwAAAA";
        assert_eq!(
            extract_state(input).unwrap(),
            "H4sIAAAAAAAA_6tWKi5JLCktVrIy1FFQyslPzk7JTM7WAwA_JkA_FwAAAA"
        );
    }

    #[test]
    fn accepts_state_value_directly() {
        assert_eq!(extract_state("abc").unwrap(), "abc");
    }
}
