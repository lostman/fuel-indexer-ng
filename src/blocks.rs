use crate::types::TxExtra;
use fuels::{
    tx::{
        field::{InputContract, MintAmount, MintAssetId, OutputContract, Script, TxPointer},
        AssetId, Bytes32,
    },
    types::Bits256,
};

mod block_indexer {
    fuels::macros::abigen!(Contract(
        name = "BlockIndexer",
        abi = "sway/scripts/block-indexer/out/debug/block-indexer-abi.json"
    ));
}

use fuel_core_client::client::FuelClient;
use fuel_core_types::{
    blockchain::primitives::BlockId, fuel_tx::Transaction, fuel_types::BlockHeight,
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
    type Item = block_indexer::FuelBlock;

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
            // let mut receipts: Vec<block_indexer::Receipt> = vec![];
            let mut tx_data: Vec<Transaction> = vec![];
            let mut tx_extra: Vec<TxExtra> = vec![];
            for id in &block.transactions {
                let tx = futures::executor::block_on(self.client.transaction(id))
                    .expect("transaction")
                    .unwrap();
                tx_data.push(tx.transaction);
                let receipts = futures::executor::block_on(self.client.receipts(id))
                    .expect(&format!("receipts for id={id}"));
                tx_extra.push(TxExtra {
                    id: (*id).into(),
                    receipts: vec![], // receipts.unwrap_or_default().to_vec(),
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

            let header = block_indexer::Header {
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

            let transactions: Vec<block_indexer::Transaction> = tx_data
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
                .collect::<Vec<Option<block_indexer::Transaction>>>();

            // TODO: 7 is a small value. This needs to be configurable. Or when
            // Vec lands it won't be a problem.
            transactions.extend(std::iter::repeat(None).take(7 - transactions.len()));

            let fb = block_indexer::FuelBlock {
                // transaction: transactions[0].as_ref().unwrap().clone(),
                header,
                transactions: transactions.try_into().unwrap(),
            };

            let b = block.header.height;

            self.height = self.height.succ().expect("Max height reached.");

            return Some(fb);
        } else {
            None
        }
    }
}

// pub struct Header {
//     pub id: BlockId,
//     pub da_height: u64,
//     pub transactions_count: u64,
//     pub message_receipt_count: u64,
//     pub transactions_root: MerkleRoot,
//     pub message_receipt_root: MerkleRoot,
//     pub height: u32,
//     pub prev_root: MerkleRoot,
//     pub time: Tai64,
//     pub application_hash: Hash,
// }

// FuelTransaction::Script(v) => transaction::Kind::Script((v, &tx_extra.receipts).into()),
// FuelTransaction::Create(v) => transaction::Kind::Create(v.into()),
// FuelTransaction::Mint(v) => transaction::Kind::Mint(v.into()),

impl From<(&fuels::tx::FuelTransaction, TxExtra)> for crate::blocks::block_indexer::Transaction {
    fn from((tx, tx_data): (&fuels::tx::FuelTransaction, TxExtra)) -> Self {
        // TODO
        match tx {
            fuels::tx::FuelTransaction::Create(create) => {
                block_indexer::Transaction::Create(create.into())
            }
            fuels::tx::FuelTransaction::Mint(mint) => {
                let m: block_indexer::Mint = mint.into();
                block_indexer::Transaction::Mint(m)
            }
            fuels::tx::FuelTransaction::Script(script) => {
                let s = ScriptWithReceipts {
                    script: script.clone(), //, tx_data.receipts
                };
                block_indexer::Transaction::Script(s.into())
            }
        }
    }
}

use fuels::tx::field::ScriptGasLimit;

struct ScriptWithReceipts {
    script: fuel_core_types::fuel_tx::Script, // tx_data: Vec<Receipt>
}

impl From<ScriptWithReceipts> for block_indexer::Script {
    fn from(ScriptWithReceipts { script }: ScriptWithReceipts) -> Self {
        Self {
            script_gas_limit: script.script_gas_limit().to_owned(),
        }
        //             script: value.script().to_vec(),
        //             script_data: value.script_data().to_vec(),
        //             policies: Some(value.policies().into()),
        //             inputs: value.inputs().iter().map(Into::into).collect(),
        //             outputs: value.outputs().iter().map(Into::into).collect(),
        //             witnesses: value
        //                 .witnesses()
        //                 .iter()
        //                 .map(|w| w.as_vec().clone())
        //                 .collect(),
        //             receipts_root: value.receipts_root().as_slice().to_owned(),
        //             receipts: receipts.iter().map(Into::into).collect(),
        //         }
    }
}

impl From<&fuel_core_types::fuel_tx::Create> for block_indexer::Create {
    fn from(value: &fuel_core_types::fuel_tx::Create) -> Self {
        //         Self {
        //             bytecode_length: *value.bytecode_length(),
        //             bytecode_witness_index: (*value.bytecode_witness_index()).into(),
        //             policies: Some(value.policies().into()),
        //             storage_slots: value.storage_slots().iter().map(Into::into).collect(),
        //             inputs: value.inputs().iter().map(Into::into).collect(),
        //             outputs: value.outputs().iter().map(Into::into).collect(),
        //             witnesses: value
        //                 .witnesses()
        //                 .iter()
        //                 .map(|w| w.as_vec().clone())
        //                 .collect(),
        //             salt: value.salt().as_slice().to_owned(),
        //         }
        unimplemented!()
    }
}

impl From<&fuel_core_types::fuel_tx::Mint> for block_indexer::Mint {
    fn from(mint: &fuel_core_types::fuel_tx::Mint) -> Self {
        block_indexer::Mint {
            tx_pointer: mint.tx_pointer().into(),
            // input_contract: mint.input_contract().into(),
            // output_contract: mint.output_contract().into(),
            mint_amount: *mint.mint_amount(),
            // mint_asset_id: mint.mint_asset_id().to_owned(),
        }
    }
}

impl From<&fuels::types::TxPointer> for block_indexer::TxPointer {
    fn from(mint: &fuels::types::TxPointer) -> Self {
        Self {
            block_height: mint.block_height().into(),
            tx_index: mint.tx_index(),
        }
    }
}

// impl From<fuels::types::UtxoId> for block_indexer::UtxoId {
//     fn from(utxoid: fuels::types::UtxoId) -> Self {
//         Self {
//             tx_id: Bits256::from(AssetId::new(utxoid.tx_id().to_owned().try_into().unwrap())),
//             output_index: utxoid.output_index(),
//         }
//     }
// }

// impl From<fuels::types::TxPointer> for block_indexer::TxPointer {
//     fn from(tx_ptr: fuels::types::TxPointer) -> Self {
//         Self {
//             block_height: tx_ptr.block_height().into(),
//             tx_index: tx_ptr.tx_index(),
//         }
//     }
// }

// impl From<&fuel_core_types::fuel_tx::input::contract::Contract> for block_indexer::InputContract {
//     fn from(contract: &fuel_core_types::fuel_tx::input::contract::Contract) -> Self {
//         block_indexer::InputContract {
//             utxo_id: contract.utxo_id.into(),
//             balance_root: Bits256::from(AssetId::new(contract.balance_root.try_into().unwrap())),
//             state_root: Bits256::from(AssetId::new(contract.state_root.try_into().unwrap())),
//             tx_pointer: contract.tx_pointer.into(),
//             contract_id: contract.contract_id,
//         }
//     }
// }

// impl From<&fuel_core_types::fuel_tx::output::contract::Contract> for block_indexer::OutputContract {
//     fn from(contract: &fuel_core_types::fuel_tx::output::contract::Contract) -> Self {
//         block_indexer::OutputContract {
//             input_index: contract.input_index,
//             balance_root: Bits256::from(AssetId::new(contract.balance_root.try_into().unwrap())),
//             state_root: Bits256::from(AssetId::new(contract.state_root.try_into().unwrap())),
//         }
//     }
// }
