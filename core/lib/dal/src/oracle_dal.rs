use crate::StorageProcessor;
use num::{rational::Ratio, BigUint};
use zksync_utils::{big_decimal_to_ratio, ratio_to_big_decimal};

pub(crate) const COEFFICIENT_PRECISION: usize = 10;

#[derive(Debug)]
pub struct OracleDal<'a, 'c> {
    pub storage: &'a mut StorageProcessor<'c>,
}

impl OracleDal<'_, '_> {
    pub async fn update_adjust_coefficient(&mut self, adjust_coefficient: &Ratio<BigUint>) {
        sqlx::query!(
            "UPDATE oracle \
            SET gas_token_adjust_coefficient = $1",
            ratio_to_big_decimal(adjust_coefficient, COEFFICIENT_PRECISION)
        )
        .execute(self.storage.conn())
        .await
        .unwrap();
    }

    pub async fn get_adjust_coefficient(&mut self) -> Ratio<BigUint> {
        let oracle = sqlx::query!("SELECT * FROM oracle")
            .fetch_one(self.storage.conn())
            .await
            .unwrap();

        big_decimal_to_ratio(&oracle.gas_token_adjust_coefficient).unwrap()
    }
}
