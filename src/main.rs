use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use anyhow::Context;

use fuel_asm::{PanicReason, RegId};
use fuel_tx::{ConsensusParameters, Finalizable, Script, TransactionBuilder, Receipt};
use fuel_vm::{
    error::SimpleResult,
    interpreter::EcalHandler,
    prelude::{Interpreter, IntoChecked, MemoryClient, MemoryRange},
    storage::MemoryStorage,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct FileReadEcal;

impl EcalHandler for FileReadEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()> {
        let a = vm.registers()[a]; // Seek offset
        let b = vm.registers()[b]; // Read length
        let c = vm.registers()[c]; // File path pointer in vm memory
        let d = vm.registers()[d]; // File path length

        vm.gas_charge(b.saturating_add(1))?;

        // Extract file path from vm memory
        let r = MemoryRange::new(c, d)?;
        let path = String::from_utf8_lossy(&vm.memory()[r.usizes()]);
        let path = PathBuf::from(path.as_ref());
        println!("path: {path:?}");

        // Seek file to correct position
        let mut file = File::open(path).map_err(|_| PanicReason::EcalError)?;
        let _ = file
            .seek(SeekFrom::Start(a))
            .map_err(|_| PanicReason::EcalError)?;

        // Allocate the buffer in the vm memory and read directly from the file into it
        vm.allocate(b)?;
        let r = MemoryRange::new(vm.registers()[RegId::HP], b)?;
        file.read(&mut vm.memory_mut()[r.usizes()])
            .map_err(|_| PanicReason::EcalError)?;

        Ok(())
    }
}

fuels::prelude::abigen!(Script(
    name = "MyScript",
    abi = "script/out/debug/script-abi.json"
));

#[allow(unused)]
async fn run_script_sdk() -> anyhow::Result<()> {
    let wallet = fuels::prelude::launch_provider_and_get_wallet().await?;
    let bin_path = "script/out/debug/script.bin";
    let script_instance = MyScript::new(wallet, bin_path);

    let script_input = MyStruct { one: 3, two: 4 };

    let result = script_instance.main(script_input).call().await?;

    assert_eq!(result.value, true);

    Ok(())
}

fn run_script_vm() -> Vec<Receipt> {
    let vm: Interpreter<MemoryStorage, Script, FileReadEcal> = Interpreter::with_memory_storage();

    let script_path = "script/out/debug/script.bin";

    let script: Vec<u8> = {
        let mut file = File::open(script_path)
            .context(script_path)
            .expect("Failed to open script");
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .context(script_path)
            .expect("Failed to read script");
        contents
    };

    let script_data: Vec<u8> = fuels::core::codec::calldata!(MyStruct { one: 1, two: 6 })
        .expect("Failed to encode struct");

    let mut client = MemoryClient::from_txtor(vm.into());
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(0)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_random_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("Failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");
    receipts.to_owned()
}

#[tokio::main]
async fn main() {
    // println!("Running script through SDK");
    // run_script_sdk().await.unwrap();

    println!("> Running script directly on the VM");
    let receipts = run_script_vm();
    println!("> Receipts");
    println!("{receipts:#?}");
}
