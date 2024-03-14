use fuels::{
    tx::{
        field::{InputContract, MintAmount, MintAssetId, OutputContract, TxPointer},
        AssetId, Receipt, TxId,
    },
    types::Bits256,
};

use fuel_core_client::client::FuelClient;
use fuel_core_types::{fuel_tx::Transaction, fuel_types::BlockHeight};

use fuels::tx::field::{
    BytecodeLength, BytecodeWitnessIndex, Inputs, Outputs, Policies, ReceiptsRoot, Salt, Script,
    ScriptData, ScriptGasLimit, StorageSlots, Witnesses,
};

pub struct BlocksIter {
    height: BlockHeight,
    client: FuelClient,
}

impl BlocksIter {
    pub fn new(height: u32) -> anyhow::Result<BlocksIter> {
        let client = FuelClient::new("beta-5.fuel.network")?;
        let height: BlockHeight = BlockHeight::new(height);
        Ok(BlocksIter { client, height })
    }
}

impl Iterator for BlocksIter {
    type Item = crate::types::sway::FuelBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(block) =
            futures::executor::block_on(self.client.block_by_height(self.height.into()))
                .expect("block_by_height")
        {
            let mut tx_data: Vec<Transaction> = vec![];
            let mut tx_extra: Vec<crate::types::TxExtra> = vec![];
            for id in &block.transactions {
                let tx = futures::executor::block_on(self.client.transaction(id))
                    .expect("transaction")
                    .unwrap();
                tx_data.push(tx.transaction);
                let receipts = futures::executor::block_on(self.client.receipts(id))
                    .expect(&format!("receipts for id={id}"));
                tx_extra.push(crate::types::TxExtra {
                    id: (*id).into(),
                    receipts: receipts.unwrap_or_default().to_vec(),
                });
            }

            let header = crate::types::sway::Header {
                block_id: Bits256::from(AssetId::new(block.id.try_into().unwrap())),
                height: block.header.height,
                da_height: block.header.da_height,
                message_receipt_count: block.header.message_receipt_count,
                transactions_count: block.header.transactions_count,
                message_receipt_root: Bits256::from(AssetId::new(
                    block.header.message_receipt_root.into(),
                )),
                prev_root: Bits256::from(AssetId::new(block.header.prev_root.into())),
                transactions_root: Bits256::from(AssetId::new(
                    block.header.transactions_root.into(),
                )),
            };

            let transactions: Vec<crate::types::sway::Transaction> = tx_data
                .iter()
                .zip(tx_extra)
                .map(|(tx, tx_extra)| (tx, tx_extra).into())
                .collect();

            use crate::types::VecExt;

            let block = crate::types::sway::FuelBlock {
                block_id: Bits256::from(AssetId::new(*block.id)),
                height: block.header.height,
                header,
                transactions: transactions.vec_to_option_array(),
            };

            self.height = self.height.succ().expect("Max height reached.");

            return Some(block);
        } else {
            None
        }
    }
}
