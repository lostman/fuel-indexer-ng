script;

use std::string::String;

use my_contract_lib::{MyStruct, MyOtherStruct, MyComplexStruct};
use ecal_lib::{println, println_str, println_u64, print_any, read_file_raw, TypeName};

struct S {
    value: (MyStruct, MyOtherStruct)
}

impl TypeName for S {
    fn type_name() -> str {
        "struct S"
    }
}

fn main(value: MyStruct) {
    println_str("MyStruct Indexer START");
    print_any(value);
    println_u64(value.one);
    println_u64(value.two);

    let mos = MyOtherStruct { value: 33 };
    let mcs = MyComplexStruct { one: value, two: mos };
    print_any(mcs);

    let s = S { value: (value, mos) };
    print_any(s);
    
    println_str("MyStruct Indexer END");
}