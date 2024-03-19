use fuel_asm::RegId;
use fuel_vm::{error::SimpleResult, interpreter::EcalHandler, prelude::Interpreter};
use fuels::core::codec::DecoderConfig;

use sqlx::{Pool, Postgres};

mod ecal_load;
mod ecal_print;
mod ecal_save;
mod ecal_type_id;

fuels::macros::abigen!(Contract(
    name = "MyContract",
    abi = "sway/scripts/mystruct-indexer/out/debug/mystruct-indexer-abi.json"
));

const DECODER_CONFIG: DecoderConfig = DecoderConfig {
    max_depth: 45,
    max_tokens: 100_000,
};

#[derive(Debug, Clone)]
pub struct MyEcal {
    pub abi: crate::ABI,
    pub db_pool: Pool<Postgres>,
}

impl EcalHandler for MyEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let a = vm.registers()[ra];
        #[cfg(debug_assertions)]
        println!("CALLING ECAL {a}");
        match a {
            3 => ecal_save::save(vm, rb),
            4 => ecal_load::load(vm, rb),
            7 => ecal_type_id::type_id(vm, rb),
            8 => ecal_print::println(vm, rb),
            _ => panic!("Unexpected ECAL function number {a}"),
        }
    }
}
