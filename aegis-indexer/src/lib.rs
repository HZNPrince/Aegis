//! aegis-indexer — real-time on-chain data ingestion via gRPC and RPC.
//! Fetches account updates from Yellowstone, parses them, caches bank/reserve data, discovers token mints,
//! polls Jupiter for prices, and persists all position updates to PostgreSQL.
pub mod grpc;
pub mod oracle;
pub mod parsers;
pub mod state;
pub mod writer;
