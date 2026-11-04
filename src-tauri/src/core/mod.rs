pub mod config;
pub mod launch;
pub mod proxy;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub stage: &'static str,
}
pub type Result<T> = std::result::Result<T, AppError>;
pub fn error(code: &'static str, stage: &'static str) -> AppError {
    AppError { code, stage }
}
