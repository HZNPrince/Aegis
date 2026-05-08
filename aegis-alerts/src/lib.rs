//! aegis-alerts — real-time alert engine, dispatch, and LLM-powered risk explanation.
//! Evaluates wallet health periodically, fires alerts when rules trip, and routes them to channels.
pub mod dispatch;
pub mod engine;
pub mod heartbeat;
pub mod llm;
