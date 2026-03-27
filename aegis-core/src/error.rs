use thiserror::Error;

/// Unified error type for the Aegis system.
#[derive(Error, Debug)]
pub enum AegisError {
    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("gRPC stream error: {0}")]
    GrpcStream(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Price feed unavailable for {asset}")]
    PriceFeedUnavailable { asset: String },

    #[error("Protocol not supported: {0}")]
    UnsupportedProtocol(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type AegisResult<T> = Result<T, AegisError>;
