script;

use std::string::String;

use ecal_lib::{Filter, TypeName};

impl TypeName for indexer::FuelBlock {
    fn type_name() -> str {
        "struct FuelBlock"
    }
}

fn main(block: indexer::FuelBlock) {
    ecal_lib::print_any(block);
    ecal_lib::save(block);
}
