use derive_more::{Display, From};

/// Substrate Client error
#[derive(Debug, Display, From)]
pub enum JustificationError {
    /// Error decoding header justification.
    #[display("error decoding justification for header")]
    JustificationDecode,
    /// Justification for header is correctly encoded, but invalid.
    #[display("bad justification for header: {_0}")]
    #[from(ignore)]
    BadJustification(String),    
}
