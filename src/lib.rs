pub mod browse;
pub mod link_state;

use cp_ast_core::sample::{generate, sample_to_text};
use std::path::Path;
use thiserror::Error;

pub const EDITOR_URL: &str = "https://manabeai.github.io/cp-ast-ecosystems/";

#[derive(Debug, Error)]
pub enum RtError {
    #[error(transparent)]
    Link(#[from] link_state::LinkStateError),
    #[error(transparent)]
    Ast(#[from] cp_ast_json::ConversionError),
    #[error("failed to generate sample: {0}")]
    Sample(#[from] cp_ast_core::sample::GenerationError),
    #[error("failed to read input file {path}: {source}")]
    ReadInputFile {
        path: String,
        source: std::io::Error,
    },
}

pub fn generate_sample_text(input: &str, seed: Option<u64>) -> Result<(u64, String), RtError> {
    let resolved_input = resolve_input(input)?;
    let state = link_state::extract_state(&resolved_input)?;
    let engine = cp_ast_json::deserialize_share_state(&state)?;
    let seed = seed.unwrap_or_else(rand::random::<u64>);
    let sample = generate(&engine, seed)?;
    Ok((seed, sample_to_text(&engine, &sample)))
}

fn resolve_input(input: &str) -> Result<String, RtError> {
    let candidate = input.trim();
    if candidate.is_empty() {
        return Ok(String::new());
    }
    let path = Path::new(candidate);
    if path.is_file() {
        let contents = std::fs::read_to_string(path).map_err(|source| RtError::ReadInputFile {
            path: candidate.to_owned(),
            source,
        })?;
        Ok(contents.trim().to_owned())
    } else {
        Ok(candidate.to_owned())
    }
}
