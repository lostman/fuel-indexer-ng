library;

pub enum Receipt {
    Call: Call,
    Return: Return,
    ReturnData: ReturnData,
}

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
    digest: u64,
    pc: u64,
    is: u64,
    data: Option<Vec<u8>>,
}

//     Panic {
//         id: ContractId,
//         reason: PanicInstruction,
//         pc: Word,
//         is: Word,
//         #[derivative(PartialEq = "ignore", Hash = "ignore")]
//         #[canonical(skip)]
//         contract_id: Option<ContractId>,
//     },

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

//     LogData {
//         id: ContractId,
//         ra: Word,
//         rb: Word,
//         ptr: Word,
//         len: Word,
//         digest: Bytes32,
//         pc: Word,
//         is: Word,
//         #[derivative(Debug(format_with = "fmt_option_truncated_hex::<16>"))]
//         #[derivative(PartialEq = "ignore", Hash = "ignore")]
//         #[canonical(skip)]
//         data: Option<Vec<u8>>,
//     },

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

//     ScriptResult {
//         result: ScriptExecutionResult,
//         gas_used: Word,
//     },

//     MessageOut {
//         sender: Address,
//         recipient: Address,
//         amount: Word,
//         nonce: Nonce,
//         len: Word,
//         digest: Bytes32,
//         #[derivative(Debug(format_with = "fmt_option_truncated_hex::<16>"))]
//         #[derivative(PartialEq = "ignore", Hash = "ignore")]
//         #[canonical(skip)]
//         data: Option<Vec<u8>>,
//     },

pub struct Mint {
    sub_id: u256,
    contract_id: ContractId,
    val: u64,
    pc: u64,
    is: u64,
}

pub struct Burn {
    sub_id: u256,
    contract_id: ContractId,
    val: u64,
    pc: u64,
    is: u64,
}