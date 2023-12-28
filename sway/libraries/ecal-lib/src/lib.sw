library;

use std::string::String;

pub fn read_file_raw(seek: u64, len: u64, path: str) -> raw_slice {
    let path_ptr = path.as_ptr();
    let path_len = path.len();
    let data = (seek, len, path_ptr, path_len);
    let ptr = __addr_of(data);
    // r_a=0: read_file ecal
    // r_b: arguments/result pointer
    asm(ptr: ptr, r_a: 0, r_c: 0, r_d: 0) {
        ecal r_a ptr r_c r_d;
    };
    asm(ptr: ptr) {
        ptr: raw_slice
    }
}

pub fn println(input: String) {
    let data = (input.as_bytes().buf.ptr(), input.as_bytes().len());
    let ptr = __addr_of(data);
    // r_a=1: println_str ecal
    asm(r_a: 1u64, r_b: ptr, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
    }
}

pub fn println_str(input: str) {
    println(String::from_ascii_str(input))
}

pub fn println_u64(input: u64) {
    // r_a=2: println_u64 ecal
    asm(r_a: 2u64, r_b: input, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
    }
}

pub fn type_id(input: str) -> u64 {
    let input = String::from_ascii_str(input);
    let data = (input.as_bytes().buf.ptr(), input.as_bytes().len());
    let ptr = __addr_of(data);
    // r_a=7: type_id ecal
    asm(r_a: 7u64, r_b: ptr, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
        r_b: u64
    }
}

pub trait TypeID {
    fn type_id() -> u64;
}

pub trait TypeName {
    fn type_name() -> str;
}

pub fn print_any<T>(t: T) where T: TypeName {
    // println_str("print_any");
    let type_name = T::type_name();
    // println_str(type_name);
    let type_id = type_id(type_name);
    // println_u64(type_id);
    // let size = __size_of_val(t);
    // println_str("size_of");
    // println_u64(size);

    let data = (type_id, __addr_of(t), __size_of_val(t));
    let ptr = __addr_of(data);
    // r_a=8: print_any ecal
    asm(r_a: 8u64, r_b: ptr, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
    }

}