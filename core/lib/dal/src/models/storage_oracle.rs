#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StorageOracle {
    pub gas_token_adjust_coeiffient: f64,
}
