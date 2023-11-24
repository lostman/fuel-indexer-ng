script;

struct MyStruct {
    one: u64,
    two: u64,
}

fn main(value: MyStruct) -> bool {
    let result = value.one + value.two;
    result == 7
}
