script;

use std::string::String;

struct MyStruct {
    one: u64,
    two: u64,
}

pub fn read_file_raw(seek: u64, len: u64, path: str) -> raw_slice {
    let path_ptr = path.as_ptr();
    let path_len = path.len();
    let data = (seek, len, path_ptr, path_len);
    let ptr = __addr_of(data);
    // ecal 0 for read_file
    let r_a = 0u64;
    // unused
    let r_c = 0u64;
    let r_d = 0u64;
    asm(ptr: ptr, r_a: r_a, r_c: r_c, r_d: r_d) {
        ecal r_a ptr r_c r_d;
    };
    let output = (0u64, 0u64);
    let out_ptr = __addr_of(output);
    ptr.copy_to::<(u64, u64)>(out_ptr, 1);
    asm(output: output) {
        output: raw_slice
    }
}

pub fn println(input: String) {
    let data = (input.as_bytes().buf.ptr(), input.as_bytes().len());
    let ptr = __addr_of(data);
    // ecal 1 for println
    let r_a = 1u64;
    // unused
    let r_c = 0u64;
    let r_d = 0u64;
    asm(r_a: r_a, r_b: ptr, r_c: r_c, r_d: r_d) {
        ecal r_a r_b r_c r_d;
    }
}

pub fn println_str(input: str) {
    println(String::from_ascii_str(input))
}

fn main(value: MyStruct) -> bool {
    println_str("START");
    let output = read_file_raw(0, 32, "test_input");
    let output = String::from(output);
    println(output);
    println_str("END");
    let result = value.one + value.two;
    result == 7
}