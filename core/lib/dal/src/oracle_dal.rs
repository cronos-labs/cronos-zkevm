use crate::StorageProcessor;

#[derive(Debug)]
pub struct OracleDal<'a, 'c> {
    pub storage: &'a mut StorageProcessor<'c>,
}

impl OracleDal<'_, '_> {
    pub fn update_adjust_coefficient(&mut self, adjust_coefficient: f64) {
        async_std::task::block_on(async {
            sqlx::query!(
                "UPDATE oracle \
                SET gas_token_adjust_coefficient = $1",
                adjust_coefficient
            )
            .execute(self.storage.conn())
            .await
            .unwrap();
        })
    }

    pub fn get_adjust_coefficient(&mut self) -> f64 {
        async_std::task::block_on(async {
            let coef = sqlx::query!("SELECT gas_token_adjust_coefficient FROM oracle")()
                .fetch_one(self.storage.conn())
                .await
                .unwrap();
            coef.to_f64()
        })
    }
}
