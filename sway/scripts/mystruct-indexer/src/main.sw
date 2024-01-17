script;

use std::string::String;

use my_contract_lib::*;
use ecal_lib::{TypeName, Filter};

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
    // let myotherstruct = MyOtherStruct {
    //     value: 34
    // };

    // let mystruct_2 = MyStruct {
    //     one: 777,
    //     two: 888,
    // };

    // let mycomplexstruct = MyComplexStruct {
    //     one: mystruct,
    //     one_one: mystruct_2,
    //     two: myotherstruct,
    //     three: 99,
    //     four: 0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20,
    //     five: true,
    //     six: std::u256::U256::from((1, 2, 3, 4)),
    // };

    // ecal_lib::print_any(mycomplexstruct);

    // ecal_lib::save(mycomplexstruct);

    // let x: MyComplexStruct = ecal_lib::load(
    //     // the filter argument does not do anything yet
    //     MyComplexStruct::one().eq(mystruct)
    // );

    // ecal_lib::print_any(x);

    let mut p = P {
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
    ecal_lib::print_any(r);

    p.p = 999;
    let q = Q {
        p_1: p,
        p_2: p,
    };
    let r = R {
        q_1: q,
        q_2: q,
    };
    ecal_lib::save(r);
}