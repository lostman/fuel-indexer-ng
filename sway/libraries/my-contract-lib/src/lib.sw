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
    two: MyOtherStruct,
    three: u64,
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

// Stored in the database

// pub struct MyComplexStructEntity {
//     one: Entity<MyStruct>,
//     two: Entity<MyOtherStruct>,
// }

// type ID = u64;

// pub struct Entity<T> {
//     id: ID,
//     value: T,
// }

// fn foo() {
//     let mystruct = MyStruct { one: 7, two: 8 };
//     let mystruct_entity: Entity<MyStruct> = Entity { id: 1, value: mystruct };

//     let myotherstruct = MyOtherStruct { value: 77 };
//     let myotherstruct_entity: Entity<MyOtherStruct> = Entity { id: 1, value: myotherstruct };
// }