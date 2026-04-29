pub mod ast_dto;
pub mod browse;
pub mod link_state;

use cp_ast_core::sample::{generate, sample_to_text};
use thiserror::Error;

pub const EDITOR_URL: &str = "https://manabeai.github.io/cp-ast-ecosystems/";

#[derive(Debug, Error)]
pub enum RtError {
    #[error(transparent)]
    Link(#[from] link_state::LinkStateError),
    #[error(transparent)]
    Ast(#[from] ast_dto::AstDtoError),
    #[error("failed to generate sample: {0}")]
    Sample(#[from] cp_ast_core::sample::GenerationError),
}

pub fn generate_sample_text(input: &str, seed: Option<u64>) -> Result<(u64, String), RtError> {
    let json = link_state::decode_input(input)?;
    let engine = ast_dto::engine_from_json(&json)?;
    let seed = seed.unwrap_or_else(rand::random::<u64>);
    let sample = generate(&engine, seed)?;
    Ok((seed, sample_to_text(&engine, &sample)))
}
