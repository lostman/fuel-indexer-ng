library;

use ecal_lib::{TypeID, TypeName, Field, PhantomData};

pub struct MyStruct {
    one: u64,
    two: u64,
}

pub struct MyOtherStruct {
    value: u32
}

pub struct MyComplexStruct {
    one: MyStruct,
    two: MyOtherStruct,
    three: u64,
    four: b256,
    five: bool,
    six: std::u256::U256,
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

impl TypeID for MyStruct {
    fn type_id() -> u64 {
        ecal_lib::type_id(Self::type_name())
    }
}

impl TypeID for MyOtherStruct {
    fn type_id() -> u64 {
        ecal_lib::type_id(Self::type_name())
    }
}

impl TypeID for MyComplexStruct {
    fn type_id() -> u64 {
        ecal_lib::type_id(Self::type_name())
    }
}

//
// These would be generated from THE ABI
//

impl MyStruct {
    pub fn value() -> Field<MyStruct, u64> {
    Field {
            field: 0,
            phantom: PhantomData::<(MyStruct, u64)>{},
        }
    }
}

impl MyComplexStruct {
    pub fn one() -> Field<MyComplexStruct, MyStruct> {
    Field {
            field: 0,
            phantom: PhantomData::<(MyComplexStruct, MyStruct)>{},
        }
    }

    pub fn two() -> Field<MyComplexStruct, MyOtherStruct> {
        Field {
            field: 1,
            phantom: PhantomData::<(MyComplexStruct, MyOtherStruct)>{},
        }
    }
    pub fn three() -> Field<MyComplexStruct, u64> {
        Field {
            field: 2,
            phantom: PhantomData::<(MyComplexStruct, u64)>{},
        }
    }
}