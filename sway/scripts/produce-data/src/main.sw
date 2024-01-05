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

    let complex = MyComplexStruct { one: value_1, two: value_2, three: 123 };
    log(complex);

    value_1
}