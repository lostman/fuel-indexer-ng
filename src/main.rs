use anyhow::Context;
use serde_json::Value;
use std::collections::HashMap;
use std::io::BufReader;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use fuel_abi_types::abi::program::TypeDeclaration;
use fuel_asm::{PanicReason, RegId};
use fuel_tx::{ConsensusParameters, Finalizable, Receipt, Script, TransactionBuilder};
use fuel_vm::{
    error::SimpleResult,
    interpreter::EcalHandler,
    prelude::{Interpreter, IntoChecked, MemoryClient, MemoryRange},
    storage::MemoryStorage,
};
use fuels::core::codec::{ABIDecoder, DecoderConfig};
use fuels::types::Token;

mod abi;

#[derive(Debug, Clone, Copy, Default)]
pub struct MyEcal;

type VM = Interpreter<MemoryStorage, Script, MyEcal>;

impl EcalHandler for MyEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        ra: RegId,
        rb: RegId,
        rc: RegId,
        rd: RegId,
    ) -> SimpleResult<()> {
        let a = vm.registers()[ra];
        match a {
            0 => Self::read_file_ecal(vm, ra, rb, rc, rd),
            1 => Self::println_str_ecal(vm, ra, rb, rc, rd),
            2 => Self::println_u64_ecal(vm, ra, rb, rc, rd),
            7 => Self::type_id_ecal(vm, ra, rb, rc, rd),
            8 => Self::print_any_ecal(vm, ra, rb, rc, rd),
            _ => panic!("Unexpected ECAL function number {a}"),
        }
    }
}

impl MyEcal {
    fn read_file_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let args: (u64, u64, u64, u64) = {
            let addr = vm.registers()[rb];
            let r = MemoryRange::new(addr, 4 * 8)?;
            let bytes: [u8; 4 * 8] = vm.memory()[r.usizes()].try_into().unwrap();
            fuels::core::codec::try_from_bytes(&bytes, fuels::core::codec::DecoderConfig::default())
                .unwrap()
        };
        println!("read_file args = {args:?}");

        vm.gas_charge(args.1.saturating_add(1))?;

        // Extract file path from vm memory
        let path = {
            let r = MemoryRange::new(args.2, args.3)?;
            let path = String::from_utf8_lossy(&vm.memory()[r.usizes()]);
            let path = PathBuf::from(path.as_ref());
            println!("read_file file_path = {path:?}");
            path
        };

        // Seek file to correct position
        let mut file = File::open(path).map_err(|_| PanicReason::EcalError)?;
        let _ = file
            .seek(SeekFrom::Start(args.0))
            .map_err(|_| PanicReason::EcalError)?;

        // Allocate the buffer in the vm memory and read directly from the file into it
        let output = {
            vm.allocate(args.1)?;
            let r: MemoryRange = MemoryRange::new(vm.registers()[RegId::HP], args.1)?;
            let len = file
                .read(&mut vm.memory_mut()[r.usizes()])
                .map_err(|_| PanicReason::EcalError)?;
            println!("read_file read {len} bytes");
            (r.start as u64, len as u64)
        };

        let output_bytes: Vec<u8> =
            fuels::core::codec::calldata!(output).expect("Failed to encode output tuple");
        vm.allocate(output_bytes.len() as u64)?;
        let o = MemoryRange::new(vm.registers()[RegId::HP], output_bytes.len())?;
        println!("output = {} {:?}", o.start, o.usizes());
        vm.memory_mut()[o.usizes()].copy_from_slice(&output_bytes);

        // Return the address of the output tuple through the rB register
        vm.registers_mut()[rb] = o.start as u64;

        Ok(())
    }

    fn println_str_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let str: String = {
            // r_b: the address of (address, lenght)
            let addr = vm.registers()[rb];
            // read the tuple stored as two consecutive u64 values
            let r = MemoryRange::new(addr, 2 * std::mem::size_of::<u64>())?;
            let bytes: [u8; 2 * std::mem::size_of::<u64>()] =
                vm.memory()[r.usizes()].try_into().unwrap();
            // convert to (address, length) of the string to be printed
            let (addr, len): (u64, u64) = fuels::core::codec::try_from_bytes(
                &bytes,
                fuels::core::codec::DecoderConfig::default(),
            )
            .unwrap();
            // read the string
            let r = MemoryRange::new(addr, len)?;
            let bytes = vm.memory()[r.usizes()].to_vec();
            String::from_utf8(bytes).unwrap()
        };
        println!("{str}");
        Ok(())
    }

    fn println_u64_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        // r_b: the value to be printed
        let value = vm.registers()[rb];
        println!("{value}");
        Ok(())
    }

    fn print_any_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let (type_id, addr, size): (u64, u64, u64) = {
            let addr = vm.registers()[rb];
            let r = MemoryRange::new(addr, 3 * 8)?;
            let bytes: [u8; 3 * 8] = vm.memory()[r.usizes()].try_into().unwrap();
            fuels::core::codec::try_from_bytes(&bytes, fuels::core::codec::DecoderConfig::default())
                .unwrap()
        };

        let data = {
            let r = MemoryRange::new(addr, size)?;
            let mut bytes = Vec::with_capacity(size as usize);
            bytes.extend_from_slice(&vm.memory()[r.usizes()]);
            bytes
        };

        // println!("print_any_ecal type_id = {type_id}");

        let param_type = crate::abi::param_type(type_id);
        let tokens = ABIDecoder::new(DecoderConfig::default())
            .decode(&param_type, data.as_ref())
            .unwrap();
        println!("> print_any = {tokens:?}");
        let decl = crate::abi::type_declaration(type_id);
        let result = pp_token(0, decl, tokens);
        println!("> print_any:");
        println!("{result}");

        Ok(())
    }

    fn type_id_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let type_name: String = {
            // r_b: the address of (address, lenght)
            let addr = vm.registers()[rb];
            // read the tuple stored as two consecutive u64 values
            let r = MemoryRange::new(addr, 2 * std::mem::size_of::<u64>())?;
            let bytes: [u8; 2 * std::mem::size_of::<u64>()] =
                vm.memory()[r.usizes()].try_into().unwrap();
            // convert to (address, length) of the string to be printed
            let (addr, len): (u64, u64) = fuels::core::codec::try_from_bytes(
                &bytes,
                fuels::core::codec::DecoderConfig::default(),
            )
            .unwrap();
            // read the string
            let r = MemoryRange::new(addr, len)?;
            let bytes = vm.memory()[r.usizes()].to_vec();
            String::from_utf8(bytes).unwrap()
        };

        let type_id = crate::abi::type_id(&type_name);

        // println!("type_id_ecal {type_name} = {type_id}");

        vm.registers_mut()[rb] = type_id as u64;

        Ok(())
    }
}

use std::convert::TryInto;

fuels::prelude::abigen!(Script(
    name = "MyScript",
    abi = "sway/scripts/produce-data/out/debug/produce-data-abi.json"
));

fn run_script(script_path: &str, script_data: Vec<u8>) -> Vec<Receipt> {
    let vm: VM = Interpreter::with_memory_storage();

    let script: Vec<u8> = {
        let mut file = File::open(script_path)
            .context(script_path.to_string())
            .expect("Failed to open script");
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .context(script_path.to_owned())
            .expect("Failed to read script");
        contents
    };

    let mut client = MemoryClient::from_txtor(vm.clone().into());
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
    client.receipts().expect("Expected receipts").to_owned()
}

fn run_produce_data() -> Vec<Receipt> {
    let script_path = "sway/scripts/produce-data/out/debug/produce-data.bin";
    let script_data: Vec<u8> = fuels::core::codec::calldata!().expect("Failed to encode struct");
    run_script(script_path, script_data)
}

fn run_indexer_script(script_path: &str, data: Vec<u8>) {
    let receipts = run_script(script_path, data);
    println!("{receipts:#?}");
}

#[tokio::main]
async fn main() {
    let indexers = HashMap::from([
        ("struct MyStruct", "mystruct-indexer"),
        // ("struct MyOtherStruct", "myotherstruct-indexer"),
    ]);
    let receipts = run_produce_data();
    println!("{receipts:#?}");

    // map(logged type id => type id)
    let logged_types_map = produce_data_logged_types_map().unwrap();
    // map(type id => type name)
    let type_id_map = produce_data_type_id_map().unwrap();

    for r in receipts {
        match r {
            Receipt::LogData { rb, data, .. } => {
                let data = data.unwrap();
                let type_id = logged_types_map.get(&rb).unwrap();
                let type_name = type_id_map.get(type_id).unwrap();

                if let Some(script_name) = indexers.get(type_name.as_str()) {
                    println!("> Running the indexer script for type {type_name}");
                    let abi_path =
                        format!("sway/scripts/{script_name}/out/debug/{script_name}-abi.json");
                    crate::abi::parse_abi(&abi_path).unwrap();
                    let script_path =
                        format!("sway/scripts/{script_name}/out/debug/{script_name}.bin");
                    run_indexer_script(&script_path, data);
                } else {
                    println!("> No indexer script for type {type_name}");
                }
            }
            _ => (),
        }
    }
}

// A simple pretty-printer.
// `ty` is the entry from "types" in the ABI
// `tok` is the decoded data `Token`
fn pp_token(indent: usize, decl: TypeDeclaration, tok: Token) -> String {
    match tok {
        Token::Unit => "()".to_string(),
        Token::U64(x) => format!("{}", x),
        Token::U32(x) => format!("{}", x),
        Token::Struct(ts) => {
            let indent = indent + 4;
            let cs = decl.components.unwrap();
            let mut result = vec![];
            for (i, tok) in ts.into_iter().enumerate() {
                let name: String = cs[i].name.clone();
                let type_id: u64 = cs[i].type_id as u64;
                let decl = crate::abi::type_declaration(type_id);
                result.push(" ".repeat(indent) + &name + " = " + &pp_token(indent, decl, tok))
            }

            let type_name = decl
                .type_field
                .strip_prefix("struct ")
                .unwrap_or(&decl.type_field);
            type_name.to_string()
                + " {\n"
                + &result.join(",\n")
                + "\n"
                + &" ".repeat(indent - 4)
                + "}"
        }
        Token::Tuple(ts) => {
            let indent = indent + 4;
            let cs = decl.components.unwrap();
            let mut result = vec![];
            for (i, tok) in ts.into_iter().enumerate() {
                let type_id: u64 = cs[i].type_id as u64;
                let decl = crate::abi::type_declaration(type_id);
                result.push(" ".repeat(indent) + &pp_token(indent, decl, tok))
            }
            "(\n".to_string() + &result.join(",\n") + "\n" + &" ".repeat(indent - 4) + ")"
        }
        _ => unimplemented!(),
    }
}

fn produce_data_type_id_map() -> anyhow::Result<HashMap<u64, String>> {
    // Open the JSON file
    let path = "sway/scripts/produce-data/out/debug/produce-data-abi.json";
    let file = File::open(path).context(path.to_string())?;
    let reader = BufReader::new(file);

    // Parse the JSON data
    let json: Value = serde_json::from_reader(reader)?;

    let pretty_json = serde_json::to_string_pretty(&json)?;

    // Print the pretty-printed JSON
    println!("{}", pretty_json);

    let mut types = HashMap::new();
    for lt in json.get("types").unwrap().as_array().unwrap() {
        let type_name = lt.get("type").unwrap().as_str().unwrap();
        let type_id = lt.get("typeId").unwrap().as_u64().unwrap();
        types.insert(type_id, type_name.to_string());
    }

    println!("> Type ID Map");
    println!("{:#?}", types);

    Ok(types)
}

fn produce_data_logged_types_map() -> anyhow::Result<HashMap<u64, u64>> {
    // Open the JSON file
    let path = "sway/scripts/produce-data/out/debug/produce-data-abi.json";
    let file = File::open(path).context(path.to_string())?;
    let reader = BufReader::new(file);

    // Parse the JSON data
    let json: Value = serde_json::from_reader(reader)?;

    let mut types = HashMap::new();
    for lt in json.get("loggedTypes").unwrap().as_array().unwrap() {
        let log_id = lt.get("logId").unwrap().as_u64().unwrap();
        let type_id = lt
            .get("loggedType")
            .unwrap()
            .get("type")
            .unwrap()
            .as_u64()
            .unwrap();
        types.insert(log_id, type_id);
    }

    println!("> Type Map");
    println!("{:#?}", types);

    Ok(types)
}
