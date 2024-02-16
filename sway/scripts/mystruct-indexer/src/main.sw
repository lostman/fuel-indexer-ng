script;

use std::string::String;

use my_contract_lib::*;
use ecal_lib::{TypeName, Filter};

struct P {
    p: u32
}

impl Eq for P {
    fn eq(x: P, y: P) -> bool {
        x.p == y.p
    }
}

struct Q {
    p_1: P,
    p_2: P,
}

impl Eq for Q {
    fn eq(x: Q, y: Q) -> bool {
        x.p_1 == y.p_1 && x.p_2 == y.p_2
    }
}

struct R {
    q_1: Q,
    q_2: Q,
}

impl Eq for R {
    fn eq(x: R, y: R) -> bool {
        x.q_1 == y.q_1 && x.q_2 == y.q_2
    }
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

struct V {
    v: [u64; 10]
}

impl TypeName for V {
    fn type_name() -> str {
        "struct V"
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

    ecal_lib::print_any(mycomplexstruct);

    ecal_lib::save(mycomplexstruct);

    let x: MyComplexStruct = ecal_lib::load(
        // the filter argument does not do anything yet
        MyComplexStruct::one().eq(mystruct)
    );

    ecal_lib::print_any(x);

    assert_eq(mycomplexstruct.one.one, x.one.one);
    assert_eq(mycomplexstruct.one.two, x.one.two);
    assert_eq(mycomplexstruct.one_one.one, x.one_one.one);
    assert_eq(mycomplexstruct.one_one.two, x.one_one.two);
    assert_eq(mycomplexstruct.two.value, x.two.value);
    assert_eq(mycomplexstruct.three, x.three);
    assert_eq(mycomplexstruct.four, x.four);
    assert_eq(mycomplexstruct.five, x.five);
    // assert_eq(mycomplexstruct.six, x.six);

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

    let r2: R = ecal_lib::load(ecal_lib::Filter::<R>::any());
    ecal_lib::print_any(r2);

    assert_eq(r, r2);

    let v = V {
        v: [0,1,2,3,4,5,6,7,8,9]
    };

    ecal_lib::save(v);
}