script;

use std::string::String;

use my_contract_lib::*;
use ecal_lib::{println, println_str, println_u64, print_any, read_file_raw, TypeName, Filter};



struct P {
    p: u32
}

struct Q {
    p_1: P,
    p_2: P,
}

struct R {
    q_1: Q,
    q_2: Q,
}

impl TypeName for P {
    fn type_name() -> str {
        "struct P"
    }
}

impl TypeName for Q {
    fn type_name() -> str {
        "struct Q"
    }
}

impl TypeName for R {
    fn type_name() -> str {
        "struct R"
    }
}

fn main(mystruct: MyStruct) {
    let myotherstruct = MyOtherStruct {
        value: 34
    };

    let mystruct_2 = MyStruct {
        one: 777,
        two: 888,
    };

    let mycomplexstruct = MyComplexStruct {
        one: mystruct,
        one_one: mystruct_2,
        two: myotherstruct,
        three: 99,
        four: 0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20,
        five: true,
        six: std::u256::U256::from((1, 2, 3, 4)),
    };

    // // Generic pretty-print ECAL
    // println_str("We're going to save this value in the database:");
    // print_any(mycomplexstruct);

    // // Store in the database ECAL
    // ecal_lib::save(mycomplexstruct);
    // println_str("...done");

    // println_str("And then load it back...");
    // // Find MyComplexStruct value such that field .one contains MyStruct value such that its field .value contains 33
    // let y: MyStruct = ecal_lib::load(MyStruct::value().eq(33));
    // print_any(y);
    // let x: MyComplexStruct = ecal_lib::load(MyComplexStruct::one().eq(y));
    // println_str("Loaded value:");
    // print_any(x);
    // println_str("...done");


    let p = P {
        p: 777,
    };

    let q = Q {
        p_1: p,
        p_2: p,
    };

    let r = R {
        q_1: q,
        q_2: q,
    };
    
    ecal_lib::save(r);
    let r: R = ecal_lib::load(ecal_lib::Filter::<R>::any());
    print_any(r);

}