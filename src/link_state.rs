use std::io::Read;

use base64::Engine;
use flate2::read::GzDecoder;
use thiserror::Error;
use url::Url;

const SHARE_STATE_PREFIX: &str = "v2.";

#[derive(Debug, Error)]
pub enum LinkStateError {
    #[error("input does not contain a state value")]
    MissingState,
    #[error("invalid URL encoding: {0}")]
    UrlDecode(String),
    #[error("invalid base64 state: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("failed to decompress state: {0}")]
    Gzip(#[from] std::io::Error),
    #[error("state is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
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
    if let Some(encoded) = state.strip_prefix(SHARE_STATE_PREFIX) {
        let compressed = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded)?;
        let mut decoder = GzDecoder::new(compressed.as_slice());
        let mut json = String::new();
        decoder.read_to_string(&mut json)?;
        return Ok(json);
    }

    let decoded =
        urlencoding::decode(state).map_err(|err| LinkStateError::UrlDecode(err.to_string()))?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(decoded.as_bytes())?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    fn encode_v2(json: &str) -> String {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        format!(
            "{SHARE_STATE_PREFIX}{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(compressed)
        )
    }

    #[test]
    fn extracts_state_from_full_url() {
        let input = "https://manabeai.github.io/cp-ast-ecosystems/?state=v2.abc";
        assert_eq!(extract_state(input).unwrap(), "v2.abc");
    }

    #[test]
    fn accepts_state_value_directly() {
        assert_eq!(extract_state("v2.abc").unwrap(), "v2.abc");
    }

    #[test]
    fn decodes_v2_gzip_base64url_state() {
        let state = encode_v2(r#"{"schema_version":1}"#);
        assert_eq!(decode_state(&state).unwrap(), r#"{"schema_version":1}"#);
    }

    #[test]
    fn decodes_legacy_url_encoded_base64_state() {
        let json = r#"{"schema_version":1}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let state = urlencoding::encode(&encoded);
        assert_eq!(decode_state(&state).unwrap(), json);
    }
}
