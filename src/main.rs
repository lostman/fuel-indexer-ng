use anyhow::Context;
use fuels::core::traits::Tokenizable;
use std::collections::HashMap;
use std::{fs::File, io::Read};

use fuel_vm::{
    prelude::{
        ConsensusParameters, Finalizable, Interpreter, IntoChecked, MemoryClient, Receipt, Script,
        TransactionBuilder,
    },
    storage::MemoryStorage,
};

mod abi;
mod blocks;
mod ecal;
mod prisma;
mod extensions;

use crate::abi::{print_abi, ABI};
use crate::ecal::MyEcal;

fn run_script(
    pool: Pool<Postgres>,
    abi: ABI,
    script_path: &str,
    script_data: Vec<u8>,
) -> Vec<Receipt> {
    let ecal = MyEcal { abi, db_pool: pool };
    let vm: Interpreter<MemoryStorage, Script, MyEcal> =
        Interpreter::with_memory_storage_and_ecal(ecal);

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

fn run_produce_data(pool: Pool<Postgres>) -> (ABI, Vec<Receipt>) {
    let abi_path = format!("sway/scripts/produce-data/out/debug/produce-data-abi.json");
    let abi = crate::abi::parse_abi(&abi_path).unwrap();

    let script_path = "sway/scripts/produce-data/out/debug/produce-data.bin";
    let script_data: Vec<u8> = fuels::core::codec::calldata!().expect("Failed to encode struct");
    let receipts = run_script(pool, abi.clone(), script_path, script_data);
    (abi, receipts)
}

async fn run_indexer_script(pool: Pool<Postgres>, script_name: &str, data: Vec<u8>) {
    let abi_path = format!("sway/scripts/{script_name}/out/debug/{script_name}-abi.json");
    let abi = crate::abi::parse_abi(&abi_path).unwrap();

    print_abi(&abi);

    // TODO: reenable
    // let prisma_schema = crate::prisma::schema_from_abi(&abi.types);
    // std::fs::write("prisma/prisma/schema.prisma", prisma_schema).unwrap();

    println!(">> DATABASE SCHEMA");
    let mut db_schema = crate::abi::SchemaConstructor::new(abi.clone());
    db_schema.process_program_abi(&abi);
    for stmt in db_schema.statements() {
        println!("{};", stmt);
        let result = sqlx::query(&format!("{stmt};"))
            .execute(&pool)
            .await
            .unwrap();
        println!("{result:#?}");
    }

    // crate::abi::set_ecal_abi(abi);
    let script_path = format!("sway/scripts/{script_name}/out/debug/{script_name}.bin");

    let receipts = run_script(pool, abi, &script_path, data);
    println!("{receipts:#?}");
}

use sqlx::{Pool, Postgres};

async fn foo(pool: Pool<Postgres>) {
    let bi = blocks::BlocksIter::new(1).unwrap();
    for b in bi {
        println!("{:#?}", b);

        let data = fuels::core::codec::ABIEncoder::encode(&[b.into_token()])
            .unwrap()
            .resolve(0);

        run_indexer_script(pool.clone(), "block-indexer", data).await;
    }
}

#[tokio::main]
async fn main() {
    // let conn_url =
    //     std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required for this example.");
    let conn_url = "postgresql://postgres:postgres@localhost";
    let pool: Pool<Postgres> = sqlx::PgPool::connect(&conn_url).await.unwrap();

    foo(pool.clone()).await;

    panic!("");

    let indexers = HashMap::from([
        ("struct MyStruct", "mystruct-indexer"),
        // ("struct MyOtherStruct", "myotherstruct-indexer"),
    ]);
    let (data_abi, data_receipts) = run_produce_data(pool.clone());
    println!("{data_receipts:#?}");

    for r in data_receipts {
        match r {
            Receipt::LogData { rb, data, .. } => {
                let data = data.unwrap();
                let type_id = data_abi.logged_types.get(&(rb as usize)).unwrap();
                let type_name = data_abi.types.get(type_id).unwrap().type_field.as_str();

                if let Some(script_name) = indexers.get(type_name) {
                    println!(">> Running '{script_name}' indexer script for type {type_name}");
                    run_indexer_script(pool.clone(), &script_name, data).await;
                } else {
                    println!(">> No indexer script for type {type_name}");
                }
            }
            _ => (),
        }
    }
}
