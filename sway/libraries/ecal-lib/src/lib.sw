library;

use std::string::String;

pub fn read_file_raw(seek: u64, len: u64, path: str) -> raw_slice {
    let path_ptr = path.as_ptr();
    let path_len = path.len();
    let data = (seek, len, path_ptr, path_len);
    let ptr = __addr_of(data);
    // r_a=0: read_file ecal
    // r_b: arguments/result pointer
    asm(r_a: 0, ptr: ptr, r_c: 0, r_d: 0) {
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

pub trait TypeID: TypeName {
    fn type_id() -> u64;
}

pub trait TypeName {
    fn type_name() -> str;
}

pub fn print_any<T>(t: T) where T: TypeName {
    // For now, logging te value is necessary to ensure it makes it to the ABI
    log(t);

    let type_name = T::type_name();
    let type_id = type_id(type_name);

    let data = (type_id, __addr_of(t), __size_of_val(t));
    let ptr = __addr_of(data);
    // r_a=8: print_any ecal
    asm(r_a: 8u64, r_b: ptr, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
    };
}

pub fn save<T>(t: T) where T: TypeName {
    // For now, logging te value is necessary to ensure it makes it to the ABI
    log(t);

    let type_name = T::type_name();
    let type_id = type_id(type_name);

    let data = (type_id, __addr_of(t), __size_of_val(t));
    let ptr = __addr_of(data);
    // r_a=3: save ecal
    asm(r_a: 3u64, r_b: ptr, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
    };
}

// TODO: return Option<T>
pub fn load<T>(_filter: Filter<T>) -> T where T: TypeName {
    let type_name = T::type_name();
    let type_id = type_id(type_name);

    // r_a=4: load ecal
    asm(r_a: 4u64, r_b: type_id, r_c: 0u64, r_d: 0u64) {
        ecal r_a r_b r_c r_d;
        r_b: T
    }
}

pub struct PhantomData<T> {}

pub struct Field<T, F> {
    field: u64,
    phantom: PhantomData<(T, F)>,
}

pub struct Filter<T> {
    phantom: PhantomData<T>,
}

impl<T> Filter<T> {
    pub fn any() -> Filter<T> {
        Filter { phantom: PhantomData::<T>{} }
    }
}

impl<T, F> Field<T, F> {
    pub fn eq(self, _val: F) -> Filter<T> {
        Filter { phantom: PhantomData::<T>{} }
    }
}