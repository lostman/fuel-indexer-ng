library;

mod receipt;

use std::string::String;

use ecal_lib::{Filter, TypeName};

// pub struct Block {
//     #[prost(bytes = "vec", tag = "1")]
//     pub id: ::prost::alloc::vec::Vec<u8>,
//     #[prost(uint32, tag = "2")]
//     pub height: u32,
//     #[prost(uint64, tag = "3")]
//     pub da_height: u64,
//     #[prost(uint64, tag = "4")]
//     pub msg_receipt_count: u64,
//     #[prost(bytes = "vec", tag = "5")]
//     pub tx_root: ::prost::alloc::vec::Vec<u8>,
//     #[prost(bytes = "vec", tag = "6")]
//     pub msg_receipt_root: ::prost::alloc::vec::Vec<u8>,
//     #[prost(bytes = "vec", tag = "7")]
//     pub prev_id: ::prost::alloc::vec::Vec<u8>,
//     #[prost(bytes = "vec", tag = "8")]
//     pub prev_root: ::prost::alloc::vec::Vec<u8>,
//     #[prost(fixed64, tag = "9")]
//     pub timestamp: u64,
//     #[prost(bytes = "vec", tag = "10")]
//     pub application_hash: ::prost::alloc::vec::Vec<u8>,
//     #[prost(message, repeated, tag = "11")]
//     pub transactions: ::prost::alloc::vec::Vec<Transaction>,
// }


pub struct Header {
    block_id: b256,
    da_height: u64,
    transactions_count: u64,
    message_receipt_count: u64,
    transactions_root: b256,
    message_receipt_root: b256,
    height: u32,
    prev_root: b256,
    // time: Tai64,
    // application_hash: Hash,
}

pub struct FuelBlock {
    // block_id: b256,
    // header: Header,
    // transactions: [Option<Transaction>; 30],
    // temporary, for testing
    transaction: Transaction,
}


pub enum Transaction {
    Script: Script,
    Create: Create,
    Mint: Mint,
}

pub struct Script {
    script_gas_limit: u64,
}
//     r#script: Vec<u8>,
//     script_data: Vec<u8>,
// //     pub(crate) policies: Policies,
// //     pub(crate) inputs: Vec<Input>,
// //     pub(crate) outputs: Vec<Output>,
// //     pub(crate) witnesses: Vec<Witness>,
//     receipts_root: b256,
// //     #[cfg_attr(feature = "serde", serde(skip))]
// //     #[derivative(PartialEq = "ignore", Hash = "ignore")]
// //     #[canonical(skip)]
// //     pub(crate) metadata: Option<ScriptMetadata>,
//     receipts: Vec<receipt::Receipt>
// }


pub struct Create {
    bytecode_length: u64,
    bytecode_witness_index: u8,
}

// pub struct Create {
//     pub(crate) bytecode_length: Word,
//     pub(crate) bytecode_witness_index: u8,
//     pub(crate) policies: Policies,
//     pub(crate) storage_slots: Vec<StorageSlot>,
//     pub(crate) inputs: Vec<Input>,
//     pub(crate) outputs: Vec<Output>,
//     pub(crate) witnesses: Vec<Witness>,
//     pub(crate) salt: Salt,
//     #[cfg_attr(feature = "serde", serde(skip))]
//     #[derivative(PartialEq = "ignore", Hash = "ignore")]
//     #[canonical(skip)]
//     pub(crate) metadata: Option<CreateMetadata>,
// }


pub struct TxPointer {
    /// Block height
    block_height: u32,
    /// Transaction index
    tx_index: u16,
}

pub struct UtxoId {
    /// transaction id
    tx_id: b256,
    /// output index
    output_index: u8,
}

pub struct InputContract {
    utxo_id: UtxoId,
    balance_root: b256,
    state_root: b256,
    tx_pointer: TxPointer,
    contract_id: ContractId,
}

pub struct OutputContract {
    /// Index of input contract.
    input_index: u8,
    /// Root of amount of coins owned by contract after transaction execution.
    balance_root: b256,
    /// State root of contract after transaction execution.
    state_root: b256,
}

pub struct Mint {
    /// The location of the transaction in the block.
    tx_pointer: TxPointer,
    /// The amount of funds minted.
    mint_amount: u64
}

//     /// The location of the transaction in the block.
//     tx_pointer: TxPointer,
//     /// The `Input::Contract` that assets are minted to.
//     input_contract: InputContract,
//     /// The `Output::Contract` that assets are being minted to.
//     output_contract: OutputContract,
//     /// The amount of funds minted.
//     mint_amount: u64,
//     /// The asset IDs corresponding to the minted amount.
//     mint_asset_id: AssetId,
//     // #[cfg_attr(feature = "serde", serde(skip))]
//     // #[derivative(PartialEq = "ignore", Hash = "ignore")]
//     // #[canonical(skip)]
//     // pub(crate) metadata: Option<MintMetadata>,
// }

// pub prev_id: ::prost::alloc::vec::Vec<u8>,
// #[prost(bytes = "vec", tag = "8")]
// pub prev_root: ::prost::alloc::vec::Vec<u8>,
// #[prost(fixed64, tag = "9")]
// pub timestamp: u64,
// #[prost(bytes = "vec", tag = "10")]
// pub application_hash: ::prost::alloc::vec::Vec<u8>,
// #[prost(message, repeated, tag = "11")]
// pub transactions: ::prost::alloc::vec::Vec<Transaction>,


// #[prost(bytes = "vec", tag = "1")]            X
// pub id: ::prost::alloc::vec::Vec<u8>,         X
// #[prost(uint32, tag = "2")]                   X
// pub height: u32,                              X
// #[prost(uint64, tag = "3")]                   X
// pub da_height: u64,                           X
// #[prost(uint64, tag = "4")]                   X
// pub msg_receipt_count: u64,                   X
// #[prost(bytes = "vec", tag = "5")]
// pub tx_root: ::prost::alloc::vec::Vec<u8>,
// #[prost(bytes = "vec", tag = "6")]
// pub msg_receipt_root: ::prost::alloc::vec::Vec<u8>,
// #[prost(bytes = "vec", tag = "7")]
// pub prev_id: ::prost::alloc::vec::Vec<u8>,
// #[prost(bytes = "vec", tag = "8")]
// pub prev_root: ::prost::alloc::vec::Vec<u8>,
// #[prost(fixed64, tag = "9")]
// pub timestamp: u64,
// #[prost(bytes = "vec", tag = "10")]
// pub application_hash: ::prost::alloc::vec::Vec<u8>,
// #[prost(message, repeated, tag = "11")]
// pub transactions: ::prost::alloc::vec::Vec<Transaction>,

impl TypeName for FuelBlock {
    fn type_name() -> str {
        "struct FuelBlock"
    }
}

impl TypeName for Header {
    fn type_name() -> str {
        "struct Header"
    }
}