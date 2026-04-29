use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrowseError {
    #[error("failed to open {url}: {source}")]
    Open { url: String, source: std::io::Error },
}

pub fn open_url(url: &str) -> Result<(), BrowseError> {
    webbrowser::open(url).map_err(|source| BrowseError::Open {
        url: url.to_owned(),
        source,
    })?;
    Ok(())
}
