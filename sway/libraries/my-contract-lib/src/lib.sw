library;

use ecal_lib::{TypeID, TypeName};

pub struct MyStruct {
    one: u64,
    two: u64,
}

pub struct MyOtherStruct {
    value: u32
}

pub struct MyComplexStruct {
    one: MyStruct,
    two: MyOtherStruct
}

impl TypeName for MyStruct {
    fn type_name() -> str {
        "struct MyStruct"
    }
}

impl TypeName for MyOtherStruct {
    fn type_name() -> str {
        "struct MyOtherStruct"
    }
}

impl TypeName for MyComplexStruct {
    fn type_name() -> str {
        "struct MyComplexStruct"
    }
}