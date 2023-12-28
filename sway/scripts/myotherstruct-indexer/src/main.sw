script;

use std::string::String;

use my_contract_lib::{MyOtherStruct};
use ecal_lib::{println_str, print_any, TypeName};

fn main(value: MyOtherStruct) {
    println_str("MyOtherStruct Indexer START");
    print_any(value);
    println_str("MyOtherStruct Indexer END");
}