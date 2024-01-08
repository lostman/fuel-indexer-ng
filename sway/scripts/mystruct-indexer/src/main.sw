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
    };

    // Generic pretty-print ECAL
    print_any(mycomplexstruct);

    // Store in the database ECAL
    ecal_lib::save(mycomplexstruct);

    // Find MyComplexStruct value such that field .one contains MyStruct value such that its field .value contains 33
    let x: MyComplexStruct = ecal_lib::load(
        MyComplexStruct::one().eq(
            ecal_lib::find(MyStruct::value().eq(33)).unwrap()
        )).unwrap();
    println_str("Loaded value:");
    print_any(x);
    
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