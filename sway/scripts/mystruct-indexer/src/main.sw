script;

use std::string::String;

use my_contract_lib::*;
use ecal_lib::{println, println_str, println_u64, print_any, read_file_raw, TypeName};

fn main(mystruct: MyStruct) {
    println_str("MyStruct Indexer START");

    let myotherstruct = MyOtherStruct {
        value: 33
    };

    let mycomplexstruct = MyComplexStruct {
        one: mystruct,
        two: myotherstruct,
        three: 99,
        four: 0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20,
    };

    // Generic pretty-print ECAL
    println_str("We're going to save this value in the database:");
    print_any(mycomplexstruct);

    // Store in the database ECAL
    ecal_lib::save(mycomplexstruct);
    println_str("...done");

    println_str("And then load it back...");
    // Find MyComplexStruct value such that field .one contains MyStruct value such that its field .value contains 33
    let y: MyStruct = ecal_lib::load(MyStruct::value().eq(33));
    print_any(y);
    let x: MyComplexStruct = ecal_lib::load(MyComplexStruct::one().eq(y));
    println_str("Loaded value:");
    print_any(x);
    println_str("...done");
    
    println_str("MyStruct Indexer END");
}



// struct S {
//     value: (MyStruct, (MyStruct, MyOtherStruct))
// }

// impl TypeName for S {
//     fn type_name() -> str {
//         "struct S"
//     }
// }

    // let s = S { value: (value, (value, mos)) };
    // print_any(s);
    // let s: S = load::<S>(1);