library;

pub enum Receipt {
    Call: Call,
    Return: Return,
    ReturnData: ReturnData,
    Panic: Panic,
    Revert: Revert,
    Log: Log,
    LogData: LogData,
    Transfer: Transfer,
    TransferOut: TransferOut,
    ScriptResult: ScriptResult,
    MessageOut: MessageOut,
    Mint: MintReceipt,
    Burn: Burn,
}

pub struct ScriptResult {
    // result: ScriptExecutionResult,
    gas_used: u64,
}

// pub enum ScriptExecutionResult {
//     Success: (),
//     Revert: (),
//     Panic: (),
//     // Generic failure case since any u64 is valid here
//     GenericFailure: u64,
// }

pub struct Call {
    id: ContractId,
    to: ContractId,
    amount: u64,
    asset_id: AssetId,
    gas: u64,
    param1: u64,
    param2: u64,
    pc: u64,
    is: u64,
}

pub struct Return {
    id: ContractId,
    val: u64,
    pc: u64,
    is: u64,
}

pub struct ReturnData {
    id: ContractId,
    ptr: u64,
    len: u64,
    digest: b256,
    pc: u64,
    is: u64,
    // data: Option<Vec<u8>>,
}

pub struct Panic {
    id: ContractId,
    // reason: PanicInstruction,
    pc: u64,
    is: u64,
    // #[derivative(PartialEq = "ignore", Hash = "ignore")]
    // #[canonical(skip)]
    contract_id: Option<ContractId>,
}

pub struct Revert {
    id: ContractId,
    ra: u64,
    pc: u64,
    is: u64,
}

pub struct Log {
    id: ContractId,
    ra: u64,
    rb: u64,
    rc: u64,
    rd: u64,
    pc: u64,
    is: u64,
}

pub struct LogData {
    id: ContractId,
    ra: u64,
    rb: u64,
    ptr: u64,
    len: u64,
    digest: b256,
    pc: u64,
    is: u64,
    // #[derivative(Debug(format_with = "fmt_option_truncated_hex::<16>"))]
    // #[derivative(PartialEq = "ignore", Hash = "ignore")]
    // #[canonical(skip)]
    // data: Option<Vec<u8>>,
}

pub struct Transfer {
    id: ContractId,
    to: ContractId,
    amount: u64,
    asset_id: AssetId,
    pc: u64,
    is: u64,
}

pub struct TransferOut {
    id: ContractId,
    to: Address,
    amount: u64,
    asset_id: AssetId,
    pc: u64,
    is: u64,
}

pub struct MessageOut {
    sender: Address,
    recipient: Address,
    amount: u64,
    // nonce: Nonce,
    len: u64,
    digest: b256,
    // #[derivative(Debug(format_with = "fmt_option_truncated_hex::<16>"))]
    // #[derivative(PartialEq = "ignore", Hash = "ignore")]
    // #[canonical(skip)]
    // data: Option<Vec<u8>>,
}

pub struct MintReceipt {
    sub_id: b256,
    contract_id: ContractId,
    val: u64,
    pc: u64,
    is: u64,
}

pub struct Burn {
    sub_id: b256,
    contract_id: ContractId,
    val: u64,
    pc: u64,
    is: u64,
}