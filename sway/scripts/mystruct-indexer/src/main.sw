script;

use std::string::String;

use my_contract_lib::{MyStruct, MyOtherStruct, MyComplexStruct};
use ecal_lib::{println, println_str, println_u64, print_any, read_file_raw, save, load, TypeName};

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
    save(mycomplexstruct);
    
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