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
    header: Header,
    transactions: [Option<Transaction>; 7],
    // temporary, for testing
    // transaction: Transaction,
}


pub enum Transaction {
    Script: Script,
    Create: Create,
    Mint: Mint,
}

pub struct Script {
    script_gas_limit: u64,
    // script_bytes: Vec<u8>,
    // script_data: Vec<u8>,
    policies: Policies,
    inputs: [Option<Input>; 7],
    outputs: [Option<Output>; 7],
    witnesses: [Option<Witness>; 7],
    receipts_root: b256,
    receipts: [Option<receipt::Receipt>; 7],
}

pub struct Policies {}

pub enum Input {
    CoinSigned: Coin,
    CoinPredicate: Coin,
    Contract: InputContract,
    MessageCoinSigned: Message,
    MessageCoinPredicate: Message,
    MessageDataSigned: Message,
    MessageDataPredicate: Message,
}

pub struct Coin
{
    utxo_id: UtxoId,
    owner: Address,
    amount: u64,
    asset_id: AssetId,
    tx_pointer: TxPointer,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub witness_index: Specification::Witness,
    maturity: u32,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub predicate_gas_used: Specification::PredicateGasUsed,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub predicate: Specification::Predicate,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub predicate_data: Specification::PredicateData,
}

pub struct Message
{
    /// The sender from the L1 chain.
    sender: Address,
    /// The receiver on the `Fuel` chain.
    recipient: Address,
    amount: u64,
    nonce: u256,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub witness_index: Specification::Witness,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub predicate_gas_used: Specification::PredicateGasUsed,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub data: Specification::Data,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub predicate: Specification::Predicate,
    // #[derivative(Debug(format_with = "fmt_as_field"))]
    // pub predicate_data: Specification::PredicateData,
}

pub enum Output {
    Coin: OutputCoin,
    Contract: OutputContract,
    Change: OutputChange,
    Variable: OutputVariable,
    ContractCreated: OutputContractCreated,
}

pub struct OutputCoin {
    to: Address,
    amount: u64,
    asset_id: AssetId,
}

pub struct OutputChange {
    to: Address,
    amount: u64,
    asset_id: AssetId,
}

pub struct OutputVariable {
    to: Address,
    amount: u64,
    asset_id: AssetId,
}

pub struct OutputContractCreated {
    contract_id: ContractId,
    state_root: u256,
}


pub struct Witness {
    // data: Vec<u8>,
}

pub struct Create {
    bytecode_length: u64,
    bytecode_witness_index: u8,
    policies: Policies,
    storage_slots: [Option<StorageSlot>; 7],
    inputs: [Option<Input>; 7],
    outputs: [Option<Output>; 7],
    witnesses: [Option<Witness>; 7],
    salt: u256,
}

pub struct StorageSlot {
    key: u256,
    value: u256,
}

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
    /// The `Input::Contract` that assets are minted to.
    input_contract: InputContract,
    /// The `Output::Contract` that assets are being minted to.
    output_contract: OutputContract,
    /// The amount of funds minted.
    mint_amount: u64,
    /// The asset IDs corresponding to the minted amount.
    mint_asset_id: AssetId,
}

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