library;

mod receipt;

use std::string::String;

use ecal_lib::{Filter, TypeName};


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
    block_id: b256,
    height: u32,
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
    script_bytes: BYTES,
    script_data: BYTES,
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
    witness_index: u8,
    maturity: u32,
    predicate_gas_used: u64,
    r#predicate: BYTES,
    predicate_data: BYTES,
}

pub struct Message
{
    /// The sender from the L1 chain.
    sender: Address,
    /// The receiver on the `Fuel` chain.
    recipient: Address,
    amount: u64,
    // TODO: u256
    nonce: b256,
    witness_index: u8,
    predicate_gas_used: u64,
    data: BYTES,
    r#predicate: BYTES,
    predicate_data: BYTES,
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
    // TODO: u256
    state_root: b256,
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
    // TODO: u256
    salt: b256,
}

pub struct StorageSlot {
    // TODO: u256
    key: b256,
    // TODO: u256
    value: b256,
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

// TODO: Until Vec<u8> is available, simulate Vec<u8>
type BYTES = [Option<u8>; 128];