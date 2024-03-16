use fuels::{
    tx::{
        field::{InputContract, MintAmount, MintAssetId, OutputContract, TxPointer},
        AssetId, Bytes32, Receipt, TxId,
    },
    types::Bits256,
};

use fuels::tx::field::{
    BytecodeLength, BytecodeWitnessIndex, Inputs, Outputs, Policies, ReceiptsRoot, Salt, Script,
    ScriptData, ScriptGasLimit, StorageSlots, Witnesses,
};

// Rust types to map from
mod fuel {
    pub use fuel_core_types::fuel_tx::{
        input::coin::CoinFull,
        input::contract::Contract as InputContract,
        input::message::FullMessage,
        output::contract::Contract as OutputContract,
        policies::{Policies, PolicyType},
        Create, Input, Mint, Output, Receipt, Script, ScriptExecutionResult, StorageSlot, Witness,
    };
}

// Sway types to map to
pub mod sway {
    fuels::macros::abigen!(Contract(
        name = "BlockIndexer",
        abi = "sway/scripts/block-indexer/out/debug/block-indexer-abi.json"
    ));
}

// Until Vec<T> is supported
pub trait VecExt<T, const N: usize> {
    fn vec_to_option_array(self) -> [Option<T>; N];
}

// Until Vec<T> is supported
impl<T: std::fmt::Debug + Clone, const N: usize> VecExt<T, N> for Vec<T> {
    #[track_caller]
    fn vec_to_option_array(self) -> [Option<T>; N] {
        let mut result = self
            .into_iter()
            .map(|x| Some(x))
            .collect::<Vec<Option<T>>>();
        if result.len() < N {
            // Fill the remainder of the array with None
            result.extend(std::iter::repeat(None).take(N - result.len()));
        } else if result.len() > N {
            // Truncate to match array's maximum size
            result.truncate(N);
        }
        result.try_into().unwrap()
    }
}

/// Extra info used for constructing blocks
pub struct TxExtra {
    pub id: TxId,
    pub receipts: Vec<Receipt>,
}

impl From<(&fuels::tx::FuelTransaction, TxExtra)> for crate::types::sway::Transaction {
    fn from((tx, tx_data): (&fuels::tx::FuelTransaction, TxExtra)) -> Self {
        // TODO
        match tx {
            fuels::tx::FuelTransaction::Create(create) => sway::Transaction::Create(create.into()),
            fuels::tx::FuelTransaction::Mint(mint) => sway::Transaction::Mint(mint.into()),
            fuels::tx::FuelTransaction::Script(script) => sway::Transaction::Script(sway::Script {
                script_gas_limit: script.script_gas_limit().to_owned(),
                script_bytes: script.script().to_vec().vec_to_option_array(),
                script_data: script.script_data().to_vec().vec_to_option_array(),
                policies: script.policies().into(),
                inputs: script
                    .inputs()
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<_>>()
                    .vec_to_option_array(),
                outputs: script
                    .outputs()
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<_>>()
                    .vec_to_option_array(),
                witnesses: script
                    .witnesses()
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<_>>()
                    .vec_to_option_array(),
                receipts_root: Bits256::from(AssetId::new(
                    script
                        .receipts_root()
                        .as_slice()
                        .to_owned()
                        .try_into()
                        .unwrap(),
                )),
                receipts: tx_data
                    .receipts
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<_>>()
                    .vec_to_option_array(),
            }),
        }
    }
}

impl From<&fuel::Policies> for sway::Policies {
    fn from(value: &fuel::Policies) -> Self {
        use strum::IntoEnumIterator;
        Self {
            // values: fuel::PolicyType::iter()
            //     .map(|policy_type| value.get(policy_type).unwrap_or_default())
            //     .collect::<Vec<_>>()
            //     .vec_to_option_array(),
        }
    }
}

impl From<&fuel::Input> for sway::Input {
    fn from(value: &fuel::Input) -> Self {
        match value {
            fuel::Input::CoinSigned(coin) => {
                sway::Input::CoinSigned((&coin.clone().into_full()).into())
            }

            fuel::Input::CoinPredicate(coin) => {
                sway::Input::CoinSigned((&coin.clone().into_full()).into())
            }

            fuel::Input::Contract(contract) => sway::Input::Contract(contract.into()),

            fuel::Input::MessageCoinSigned(message) => {
                sway::Input::MessageCoinSigned((&message.clone().into_full()).into())
            }

            fuel::Input::MessageCoinPredicate(message) => {
                sway::Input::MessageCoinPredicate((&message.clone().into_full()).into())
            }

            fuel::Input::MessageDataSigned(message) => {
                sway::Input::MessageDataSigned((&message.clone().into_full()).into())
            }

            fuel::Input::MessageDataPredicate(message) => {
                sway::Input::MessageDataPredicate((&message.clone().into_full()).into())
            }
        }
    }
}

impl From<&fuel::CoinFull> for sway::Coin {
    fn from(value: &fuel::CoinFull) -> Self {
        Self {
            utxo_id: value.utxo_id.into(),
            owner: value.owner.into(),
            amount: value.amount,
            asset_id: value.asset_id,
            tx_pointer: value.tx_pointer.into(),
            witness_index: value.witness_index,
            maturity: value.maturity.into(),
            predicate_gas_used: value.predicate_gas_used,
            predicate: value.predicate.clone().vec_to_option_array(),
            predicate_data: value.predicate_data.clone().vec_to_option_array(),
        }
    }
}

impl From<&fuel::FullMessage> for sway::Message {
    fn from(value: &fuel::FullMessage) -> Self {
        Self {
            sender: value.sender,
            recipient: value.recipient,
            amount: value.amount,
            nonce: Bits256::from(AssetId::new(value.nonce.as_slice().try_into().unwrap())),
            witness_index: value.witness_index,
            predicate_gas_used: value.predicate_gas_used,
            data: value.data.clone().vec_to_option_array(),
            predicate: value.predicate.clone().vec_to_option_array(),
            predicate_data: value.predicate_data.clone().vec_to_option_array(),
        }
    }
}

impl From<&fuel::Output> for sway::Output {
    fn from(value: &fuel::Output) -> Self {
        match value.clone() {
            fuel::Output::Coin {
                to,
                amount,
                asset_id,
            } => sway::Output::Coin(sway::OutputCoin {
                to,
                amount,
                asset_id,
            }),
            fuel::Output::Contract(ref contract) => sway::Output::Contract(contract.into()),
            fuel::Output::Change {
                to,
                amount,
                asset_id,
            } => sway::Output::Change(sway::OutputChange::new(to, amount, asset_id)),
            fuel::Output::Variable {
                to,
                amount,
                asset_id,
            } => sway::Output::Variable(sway::OutputVariable::new(to, amount, asset_id)),

            fuel::Output::ContractCreated {
                contract_id,
                state_root,
            } => sway::Output::ContractCreated(sway::OutputContractCreated::new(
                contract_id,
                Bits256::from(AssetId::new(state_root.as_slice().try_into().unwrap())),
            )),
        }
    }
}

impl From<&fuel::Witness> for sway::Witness {
    fn from(value: &fuel::Witness) -> Self {
        Self {
            data: value.as_vec().clone().vec_to_option_array(),
        }
    }
}

impl From<&fuel::Create> for sway::Create {
    fn from(value: &fuel::Create) -> Self {
        Self {
            bytecode_length: *value.bytecode_length(),
            bytecode_witness_index: (*value.bytecode_witness_index()).into(),
            policies: value.policies().into(),
            storage_slots: value
                .storage_slots()
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .vec_to_option_array(),
            inputs: value
                .inputs()
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .vec_to_option_array(),
            outputs: value
                .outputs()
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .vec_to_option_array(),
            witnesses: value
                .witnesses()
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .vec_to_option_array(),
            salt: Bits256::from(AssetId::new(value.salt().as_slice().try_into().unwrap())),
        }
    }
}

impl From<&fuel::Mint> for sway::Mint {
    fn from(mint: &fuel::Mint) -> Self {
        sway::Mint {
            tx_pointer: mint.tx_pointer().into(),
            input_contract: mint.input_contract().into(),
            output_contract: mint.output_contract().into(),
            mint_amount: *mint.mint_amount(),
            mint_asset_id: mint.mint_asset_id().to_owned(),
        }
    }
}

impl From<&fuels::types::TxPointer> for sway::TxPointer {
    fn from(mint: &fuels::types::TxPointer) -> Self {
        Self {
            block_height: mint.block_height().into(),
            tx_index: mint.tx_index(),
        }
    }
}

impl From<fuels::types::UtxoId> for sway::UtxoId {
    fn from(utxoid: fuels::types::UtxoId) -> Self {
        Self {
            tx_id: Bits256::from(AssetId::new(utxoid.tx_id().to_owned().try_into().unwrap())),
            output_index: utxoid.output_index(),
        }
    }
}

impl From<fuels::types::TxPointer> for sway::TxPointer {
    fn from(tx_ptr: fuels::types::TxPointer) -> Self {
        Self {
            block_height: tx_ptr.block_height().into(),
            tx_index: tx_ptr.tx_index(),
        }
    }
}

impl From<&fuel::StorageSlot> for sway::StorageSlot {
    fn from(value: &fuel::StorageSlot) -> Self {
        sway::StorageSlot {
            key: Bits256::from(AssetId::new(value.key().as_ref().try_into().unwrap())),
            value: Bits256::from(AssetId::new(value.value().as_ref().try_into().unwrap())),
        }
    }
}

impl From<&fuel::InputContract> for sway::InputContract {
    fn from(contract: &fuel::InputContract) -> Self {
        sway::InputContract {
            utxo_id: contract.utxo_id.into(),
            balance_root: Bits256::from(AssetId::new(contract.balance_root.try_into().unwrap())),
            state_root: Bits256::from(AssetId::new(contract.state_root.try_into().unwrap())),
            tx_pointer: contract.tx_pointer.into(),
            contract_id: contract.contract_id,
        }
    }
}

impl From<&fuel::OutputContract> for sway::OutputContract {
    fn from(contract: &fuel::OutputContract) -> Self {
        sway::OutputContract {
            input_index: contract.input_index,
            balance_root: Bits256::from(AssetId::new(contract.balance_root.try_into().unwrap())),
            state_root: Bits256::from(AssetId::new(contract.state_root.try_into().unwrap())),
        }
    }
}

impl From<&fuel::Receipt> for sway::Receipt {
    fn from(receipt: &fuel::Receipt) -> Self {
        match receipt.clone() {
            fuel::Receipt::Call {
                id,
                to,
                amount,
                asset_id,
                gas,
                param1,
                param2,
                pc,
                is,
            } => Self::Call(sway::Call {
                id,
                to,
                amount,
                asset_id,
                gas,
                param_1: param1,
                param_2: param2,
                pc,
                is,
            }),
            fuel::Receipt::Return { id, val, pc, is } => {
                Self::Return(sway::Return { id, val, pc, is })
            }
            fuel::Receipt::ReturnData {
                id,
                ptr,
                len,
                digest,
                pc,
                is,
                data,
            } => Self::ReturnData(sway::ReturnData {
                id,
                ptr,
                len,
                digest: Bits256::from(AssetId::new(digest.to_vec().try_into().unwrap())),
                pc,
                is,
            }),
            fuel::Receipt::ScriptResult { result, gas_used } => {
                Self::ScriptResult(sway::ScriptResult {
                    // result: (&result).into(),
                    gas_used,
                })
            }
            fuel::Receipt::MessageOut {
                sender,
                recipient,
                amount,
                nonce,
                len,
                digest,
                data,
            } => Self::MessageOut(sway::MessageOut {
                sender,
                recipient,
                amount,
                // nonce,
                digest: Bits256::from(AssetId::new(digest.as_slice().try_into().unwrap())),
                len,
            }),
            fuel::Receipt::Mint {
                sub_id,
                contract_id,
                val,
                pc,
                is,
            } => Self::Mint(sway::MintReceipt {
                sub_id: Bits256::from(AssetId::new(sub_id.as_slice().try_into().unwrap())),
                contract_id,
                val,
                pc,
                is,
            }),

            fuel::Receipt::Revert { id, ra, pc, is } => {
                Self::Revert(sway::Revert { id, ra, pc, is })
            }

            fuel::Receipt::Panic {
                id,
                reason,
                pc,
                is,
                contract_id,
            } => Self::Panic(sway::Panic {
                id,
                // reason: reason.into(),
                pc,
                is,
                contract_id,
            }),

            fuel::Receipt::TransferOut {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            } => Self::TransferOut(sway::TransferOut {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            }),

            fuel::Receipt::LogData {
                id,
                ra,
                rb,
                ptr,
                len,
                digest,
                pc,
                is,
                data,
            } => Self::LogData(sway::LogData {
                id,
                ra,
                rb,
                ptr,
                len,
                digest: digest.to_bits256(),
                pc,
                is,
                // data,
            }),

            fuel::Receipt::Log {
                id,
                ra,
                rb,
                rc,
                rd,
                pc,
                is,
            } => Self::Log(sway::Log {
                id,
                ra,
                rb,
                rc,
                rd,
                pc,
                is,
            }),

            fuel::Receipt::Burn {
                sub_id,
                contract_id,
                val,
                pc,
                is,
            } => Self::Burn(sway::Burn {
                sub_id: sub_id.to_bits256(),
                contract_id,
                val,
                pc,
                is,
            }),

            fuel::Receipt::Transfer {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            } => Self::Transfer(sway::Transfer {
                id,
                to,
                amount,
                asset_id,
                pc,
                is,
            }),
        }
    }
}

trait Bytes32Ext {
    fn to_bits256(&self) -> Bits256;
}

impl Bytes32Ext for Bytes32 {
    fn to_bits256(&self) -> Bits256 {
        Bits256::from(AssetId::new(self.as_slice().try_into().unwrap()))
    }
}

// impl From<&fuel::ScriptExecutionResult> for sway::ScriptExecutionResult {
//     fn from(value: &fuel::ScriptExecutionResult) -> Self {
//         match value {
//             fuel::ScriptExecutionResult::Success => sway::ScriptExecutionResult::Success,
//             fuel::ScriptExecutionResult::Revert => sway::ScriptExecutionResult::Revert,
//             fuel::ScriptExecutionResult::Panic => sway::ScriptExecutionResult::Panic,
//             fuel::ScriptExecutionResult::GenericFailure(n) => {
//                 sway::ScriptExecutionResult::GenericFailure(*n)
//             }
//         }
//     }
// }
