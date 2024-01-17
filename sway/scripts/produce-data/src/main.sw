script;

use std::string::String;

use my_contract_lib::{MyStruct, MyOtherStruct, MyComplexStruct};

fn main() -> MyStruct {
    let value_1 = MyStruct { one: 123, two: 234 };
    log(value_1);

    let value_2 = MyOtherStruct { value: 77 };
    log(value_2);

    log((value_1, value_2));

    log((value_2, value_1));

    let complex = MyComplexStruct {
        one: value_1,
        one_one: value_1,
        two: value_2,
        three: 123,
        four: 0x0000000000060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20,
        five: true,
        six: std::u256::U256::from((1, 2, 3, 4)),
    };
    log(complex);

    value_1
}