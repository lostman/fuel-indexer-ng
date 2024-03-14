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
            // let prev_id: BlockId = match self.height.pred() {
            //     Some(h) => futures::executor::block_on(self.client.block_by_height(h.into()))
            //         .expect("block_by_height")
            //         .map(|b| b.id.into())
            //         .unwrap_or_default(),
            //     None => BlockId::default(),
            // };

            // TODO: receipts
            // let mut receipts: Vec<sway::Receipt> = vec![];
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

            // id: block.id.as_slice().to_owned(),
            // height: block.header.height,
            // da_height: block.header.da_height,
            // msg_receipt_count: block.header.message_receipt_count,
            // tx_root: block.header.transactions_root.as_slice().to_owned(),
            // msg_receipt_root: block.header.message_receipt_root.as_slice().to_owned(),
            // prev_id: prev_id.as_slice().to_owned(),
            // prev_root: block.header.prev_root.as_slice().to_owned(),
            // timestamp: block.header.time.0,
            // application_hash: block.header.application_hash.to_vec(),
            // transactions: tx_data
            //     .iter()
            //     .zip(tx_extra)
            //     .map(|(tx, tx_extra)| (tx, tx_extra).into())
            //     .collect(),

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

            // Since we are simulating Vec<T> with [Option<T>; 1000], we need to
            // convert the values we have to Some(t) and extend the Vec with
            // Nones, and then convert.

            let mut transactions = transactions
                .into_iter()
                .map(|x| Some(x))
                .collect::<Vec<Option<crate::types::sway::Transaction>>>();

            // TODO: 7 is a small value. This needs to be configurable. Or when
            // Vec lands it won't be a problem.
            transactions.extend(std::iter::repeat(None).take(7 - transactions.len()));

            let fb = crate::types::sway::FuelBlock {
                block_id: Bits256::from(AssetId::new(*block.id)),
                height: block.header.height,
                header,
                transactions: transactions.try_into().unwrap(),
            };

            self.height = self.height.succ().expect("Max height reached.");

            return Some(fb);
        } else {
            None
        }
    }
}