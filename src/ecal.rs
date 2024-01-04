use std::convert::TryInto;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use fuel_abi_types::abi::program::TypeDeclaration;
use fuel_asm::{PanicReason, RegId};
use fuel_vm::{
    error::SimpleResult,
    interpreter::EcalHandler,
    prelude::{Interpreter, MemoryRange},
};
use fuels::core::codec::{ABIDecoder, DecoderConfig};
use fuels::types::Token;

#[derive(Debug, Clone, Copy, Default)]
pub struct MyEcal;

impl EcalHandler for MyEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        ra: RegId,
        rb: RegId,
        rc: RegId,
        rd: RegId,
    ) -> SimpleResult<()> {
        let a = vm.registers()[ra];
        match a {
            0 => Self::read_file_ecal(vm, ra, rb, rc, rd),
            1 => Self::println_str_ecal(vm, ra, rb, rc, rd),
            2 => Self::println_u64_ecal(vm, ra, rb, rc, rd),
            7 => Self::type_id_ecal(vm, ra, rb, rc, rd),
            8 => Self::print_any_ecal(vm, ra, rb, rc, rd),
            _ => panic!("Unexpected ECAL function number {a}"),
        }
    }
}

impl MyEcal {
    fn read_file_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let args: (u64, u64, u64, u64) = {
            let addr = vm.registers()[rb];
            let r = MemoryRange::new(addr, 4 * 8)?;
            let bytes: [u8; 4 * 8] = vm.memory()[r.usizes()].try_into().unwrap();
            fuels::core::codec::try_from_bytes(&bytes, fuels::core::codec::DecoderConfig::default())
                .unwrap()
        };
        println!("read_file args = {args:?}");

        vm.gas_charge(args.1.saturating_add(1))?;

        // Extract file path from vm memory
        let path = {
            let r = MemoryRange::new(args.2, args.3)?;
            let path = String::from_utf8_lossy(&vm.memory()[r.usizes()]);
            let path = PathBuf::from(path.as_ref());
            println!("read_file file_path = {path:?}");
            path
        };

        // Seek file to correct position
        let mut file = File::open(path).map_err(|_| PanicReason::EcalError)?;
        let _ = file
            .seek(SeekFrom::Start(args.0))
            .map_err(|_| PanicReason::EcalError)?;

        // Allocate the buffer in the vm memory and read directly from the file into it
        let output = {
            vm.allocate(args.1)?;
            let r: MemoryRange = MemoryRange::new(vm.registers()[RegId::HP], args.1)?;
            let len = file
                .read(&mut vm.memory_mut()[r.usizes()])
                .map_err(|_| PanicReason::EcalError)?;
            println!("read_file read {len} bytes");
            (r.start as u64, len as u64)
        };

        let output_bytes: Vec<u8> =
            fuels::core::codec::calldata!(output).expect("Failed to encode output tuple");
        vm.allocate(output_bytes.len() as u64)?;
        let o = MemoryRange::new(vm.registers()[RegId::HP], output_bytes.len())?;
        println!("output = {} {:?}", o.start, o.usizes());
        vm.memory_mut()[o.usizes()].copy_from_slice(&output_bytes);

        // Return the address of the output tuple through the rB register
        vm.registers_mut()[rb] = o.start as u64;

        Ok(())
    }

    fn println_str_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let str: String = {
            // r_b: the address of (address, lenght)
            let addr = vm.registers()[rb];
            // read the tuple stored as two consecutive u64 values
            let r = MemoryRange::new(addr, 2 * std::mem::size_of::<u64>())?;
            let bytes: [u8; 2 * std::mem::size_of::<u64>()] =
                vm.memory()[r.usizes()].try_into().unwrap();
            // convert to (address, length) of the string to be printed
            let (addr, len): (u64, u64) = fuels::core::codec::try_from_bytes(
                &bytes,
                fuels::core::codec::DecoderConfig::default(),
            )
            .unwrap();
            // read the string
            let r = MemoryRange::new(addr, len)?;
            let bytes = vm.memory()[r.usizes()].to_vec();
            String::from_utf8(bytes).unwrap()
        };
        println!("{str}");
        Ok(())
    }

    fn println_u64_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        // r_b: the value to be printed
        let value = vm.registers()[rb];
        println!("{value}");
        Ok(())
    }

    fn print_any_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let (type_id, addr, size): (u64, u64, u64) = {
            let addr = vm.registers()[rb];
            let r = MemoryRange::new(addr, 3 * 8)?;
            let bytes: [u8; 3 * 8] = vm.memory()[r.usizes()].try_into().unwrap();
            fuels::core::codec::try_from_bytes(&bytes, fuels::core::codec::DecoderConfig::default())
                .unwrap()
        };
        let type_id = type_id as usize;

        let data = {
            let r = MemoryRange::new(addr, size)?;
            let mut bytes = Vec::with_capacity(size as usize);
            bytes.extend_from_slice(&vm.memory()[r.usizes()]);
            bytes
        };

        // println!("print_any_ecal type_id = {type_id}");

        let param_type = crate::abi::param_type(type_id);
        let tokens = ABIDecoder::new(DecoderConfig::default())
            .decode(&param_type, data.as_ref())
            .unwrap();
        println!("> print_any = {tokens:?}");
        let result = pretty_print(type_id, tokens);
        println!("> print_any:");
        println!("{result}");

        Ok(())
    }

    fn type_id_ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let type_name: String = {
            // r_b: the address of (address, lenght)
            let addr = vm.registers()[rb];
            // read the tuple stored as two consecutive u64 values
            let r = MemoryRange::new(addr, 2 * std::mem::size_of::<u64>())?;
            let bytes: [u8; 2 * std::mem::size_of::<u64>()] =
                vm.memory()[r.usizes()].try_into().unwrap();
            // convert to (address, length) of the string to be printed
            let (addr, len): (u64, u64) = fuels::core::codec::try_from_bytes(
                &bytes,
                fuels::core::codec::DecoderConfig::default(),
            )
            .unwrap();
            // read the string
            let r = MemoryRange::new(addr, len)?;
            let bytes = vm.memory()[r.usizes()].to_vec();
            String::from_utf8(bytes).unwrap()
        };

        let type_id = crate::abi::type_id(&type_name);

        vm.registers_mut()[rb] = type_id as u64;

        Ok(())
    }
}

// Given a type id and encoded data, it pretty-prints the data.
fn pretty_print(type_id: usize, tok: Token) -> String {
    fn pretty_print_inner(indent: usize, decl: TypeDeclaration, tok: Token) -> String {
        match tok {
            Token::Unit => "()".to_string(),
            Token::U64(x) => format!("{}", x),
            Token::U32(x) => format!("{}", x),
            Token::Struct(fields) => {
                let indent = indent + 4;
                let comps = decl.components.unwrap();
                let mut result = vec![];
                for (i, field) in fields.into_iter().enumerate() {
                    let name: String = comps[i].name.clone();
                    let type_id: usize = comps[i].type_id;
                    let decl = crate::abi::type_declaration(type_id);
                    result
                        .push(" ".repeat(indent) + &name + " = " + &pretty_print_inner(indent, decl, field))
                }

                let type_name = decl
                    .type_field
                    .strip_prefix("struct ")
                    .unwrap_or(&decl.type_field);
                type_name.to_string()
                    + " {\n"
                    + &result.join(",\n")
                    + "\n"
                    + &" ".repeat(indent - 4)
                    + "}"
            }
            Token::Tuple(fields) => {
                let indent = indent + 4;
                let comps = decl.components.unwrap();
                let mut result = vec![];
                for (i, field) in fields.into_iter().enumerate() {
                    let type_id: usize = comps[i].type_id as usize;
                    let decl = crate::abi::type_declaration(type_id);
                    result.push(" ".repeat(indent) + &pretty_print_inner(indent, decl, field))
                }
                "(\n".to_string() + &result.join(",\n") + "\n" + &" ".repeat(indent - 4) + ")"
            }
            _ => unimplemented!(),
        }
    }
    let decl = crate::abi::type_declaration(type_id);
    pretty_print_inner(0, decl, tok)
}
