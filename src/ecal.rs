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
use fuels::core::codec::{ABIDecoder, ABIEncoder, DecoderConfig};
use fuels::types::param_types::ParamType;
use fuels::types::Token;

fuels::macros::abigen!(Contract(
    name = "MyContract",
    abi = "sway/scripts/mystruct-indexer/out/debug/mystruct-indexer-abi.json"
));

use sqlx::{Pool, Postgres, Row};

use std::sync::Mutex;

lazy_static::lazy_static! {
    pub static ref DB: Mutex<Option<Pool<Postgres>>> = Mutex::new(None);
}

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
            3 => Self::save(vm, ra, rb, rc, rd),
            4 => Self::load(vm, ra, rb, rc, rd),
            7 => Self::type_id_ecal(vm, ra, rb, rc, rd),
            8 => Self::print_any_ecal(vm, ra, rb, rc, rd),
            _ => panic!("Unexpected ECAL function number {a}"),
        }
    }
}

impl MyEcal {
    fn save<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        println!(">> ECAL::save()");
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

        let param_type = crate::abi::param_type(type_id);
        let tokens = ABIDecoder::new(DecoderConfig::default())
            .decode(&param_type, data.as_ref())
            .unwrap();
        // println!("> save = {tokens:?}");
        let mut stmts = save_any(type_id, tokens);
        let last = stmts.pop().unwrap();
        let stmts = stmts.join(", ");
        let stmt = format!("WITH {stmts} {last}");
        println!(">> SAVE_STMT\n{stmt}");
        let rows_affected = futures::executor::block_on(
            sqlx::query(&stmt).execute(DB.lock().unwrap().as_ref().unwrap()),
        )
        .unwrap()
        .rows_affected();
        println!(">> ROWS_AFFECTED {rows_affected}");

        Ok(())
    }

    fn load<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        _ra: RegId,
        rb: RegId,
        _rc: RegId,
        _rd: RegId,
    ) -> SimpleResult<()> {
        let type_id = vm.registers()[rb];
        println!("> ECAL::load(type_id={type_id})");

        let struct_name = crate::abi::type_declaration(type_id as usize)
            .type_field
            .strip_prefix("struct ")
            .unwrap()
            .to_string();
        let (selects, joins, types) = load_any(type_id as usize);
        let selects = selects.join(", ");
        let joins = joins.join(" ");

        let types: Vec<ParamType> = types.iter().map(|t| crate::abi::param_type(*t)).collect();
        // TODO: until `load` accepts filter parameter, return a single value as a proof of concept
        let query_string = format!("SELECT {selects} FROM \"{struct_name}\" {joins} LIMIT 1");

        println!("LOAD_QUERY_STRING:\n{query_string}");

        let query = sqlx::query(&query_string);
        // let rt = tokio::runtime::Handle::current();
        let row: sqlx::postgres::PgRow =
            futures::executor::block_on(query.fetch_one(DB.lock().unwrap().as_ref().unwrap()))
                .unwrap();

        let mut tokens = VecDeque::new();
        for (index, t) in types.iter().enumerate() {
            let tok = match t {
                ParamType::U64 => Token::U64(row.get::<i64, usize>(index) as u64),
                ParamType::U32 => Token::U32(row.get::<i32, usize>(index) as u32),
                _ => unimplemented!(),
            };
            tokens.push_back(tok);
        }

        let decl = crate::abi::type_declaration(type_id as usize);

        fn convert(
            index: &mut usize,
            row: &sqlx::postgres::PgRow,
            decl: TypeDeclaration,
            params: &mut VecDeque<ParamType>,
        ) -> Token {
            // println!("CONVERT: {index} {decl:#?} {params:#?}");
            if is_struct(&decl) {
                let mut struct_tokens = vec![];
                for field in decl.components.unwrap().iter() {
                    let field_decl = crate::abi::type_declaration(field.type_id);
                    let field_tokens = convert(index, row, field_decl, params);
                    struct_tokens.push(field_tokens)
                }
                Token::Struct(struct_tokens)
            } else {
                let field_token = match params.pop_front().unwrap() {
                    ParamType::U64 => Token::U64(row.get::<i64, usize>(*index) as u64),
                    ParamType::U32 => Token::U32(row.get::<i32, usize>(*index) as u32),
                    _ => unimplemented!(),
                };
                *index += 1;
                field_token
            }
        }

        let struct_token = convert(&mut 0, &row, decl, &mut types.into());
        // println!("XX: {struct_token:#?}");
        // println!("{}", pretty_print(type_id as usize, struct_token.clone()));
        // println!("XX_DONE");

        let output_bytes = ABIEncoder::encode(&vec![struct_token]).unwrap().resolve(0);

        vm.allocate(output_bytes.len() as u64)?;
        let o = MemoryRange::new(vm.registers()[RegId::HP], output_bytes.len())?;
        vm.memory_mut()[o.usizes()].copy_from_slice(&output_bytes);

        // Return the address of the output tuple through the rB register
        vm.registers_mut()[rb] = o.start as u64;

        Ok(())
    }

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
        // println!("> print_any = {tokens:?}");
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
                    result.push(
                        " ".repeat(indent)
                            + &name
                            + " = "
                            + &pretty_print_inner(indent, decl, field),
                    )
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

// WITH
//   MyStruct as (select one, two from mystruct),
//   MyOtherStruct as (select (value) from myotherstruct) (select * from MyStruct, MyOtherStruct);

use std::collections::{BTreeMap, VecDeque};

fn load_any(type_id: usize) -> (Vec<String>, Vec<String>, Vec<usize>) {
    let decl = crate::abi::type_declaration(type_id);

    let mut struct_columns: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut selects: Vec<String> = vec![];
    let mut joins: Vec<String> = vec![];
    let mut types: Vec<usize> = vec![];

    let struct_name = decl.type_field.strip_prefix("struct ").unwrap().to_string();
    let columns: Vec<String> = decl
        .components
        .as_ref()
        .unwrap()
        .iter()
        .map(|field| field.name.clone())
        .collect();
    struct_columns.insert(struct_name.clone(), columns);

    for field in decl.components.as_ref().unwrap().iter() {
        let field_decl = crate::abi::type_declaration(field.type_id);
        if is_struct(&field_decl) {
            let field_struct_name = field_decl.type_field.strip_prefix("struct ").unwrap();
            joins.push(format!(
                    "LEFT JOIN \"{field_struct_name}\" ON \"{struct_name}\".\"{field_name}Id\" = \"{field_struct_name}\".id",
                    field_name = field.name
                ));
            let (nested_selects, nested_joins, nested_types) = load_any(field.type_id);
            // println!("NESTED:\n{nested_selects:#?}\n{nested_joins:#?}");

            selects.extend(nested_selects);
            joins.extend(nested_joins);
            types.extend(nested_types);
        } else {
            selects.push(format!("\"{struct_name}\".\"{}\"", field.name));
            types.push(field.type_id);
        }
    }

    // println!("> LOAD_ANY:");
    // println!("{:#?}", struct_queries);
    // println!("{:#?}", selects);
    (selects, joins, types)
}

// WITH
//   id_a AS (insert INTO table_a (one) values (434) RETURNING id),
//   id_b AS (insert INTO table_b (two) VALUES (123) RETURNING id)
//   (SELECT id_a.id as id_a, id_b.id as id_b, "Some Other Value" FROM id_a, id_b);

fn save_any(type_id: usize, tok: Token) -> Vec<String> {
    let toks = if let Token::Struct(toks) = tok {
        toks
    } else {
        panic!("Expected Token::Struct argument but got {tok:#?}");
    };
    // TODO: do not need a stack here. `save_any` is recursive
    let decl = crate::abi::type_declaration(type_id);
    let mut stmts = vec![];
    let struct_name = decl.type_field.strip_prefix("struct ").unwrap();
    let columns: Vec<String> = decl
        .components
        .as_ref()
        .unwrap()
        .iter()
        .map(|field| {
            if is_struct(&crate::abi::type_declaration(field.type_id)) {
                format!("\"{}Id\"", field.name)
            } else {
                format!("\"{}\"", field.name)
            }
        })
        .collect();
    let columns = columns.join(", ");
    if has_nested_struct(&decl) {
        let mut selects: Vec<String> = vec![];
        let mut sources: Vec<String> = vec![];
        for (i, field) in decl.components.as_ref().unwrap().iter().enumerate() {
            let field_decl = crate::abi::type_declaration(field.type_id);
            if is_struct(&field_decl) {
                let field_struct_name = field_decl.type_field.strip_prefix("struct ").unwrap();
                let nested_stmts = save_any(field_decl.type_id, toks[i].clone());
                stmts.push(nested_stmts);
                let field_name = if is_struct(&field_decl) {
                    format!("{}Id", field.name)
                } else {
                    field.name.clone()
                };
                selects.push(format!("{field_struct_name}_id.id AS {field_name}"));
                sources.push(format!("{field_struct_name}_id"))
            } else {
                selects.push(tok_to_string(&toks[i]))
            }
        }
        let selects = selects.join(", ");
        let sources = sources.join(", ");
        let stmt = format!("INSERT INTO \"{struct_name}\" ({columns}) (SELECT {selects} FROM {sources}) RETURNING id");
        stmts.push(vec![stmt]);
    } else {
        let values: Vec<String> = toks.iter().map(tok_to_string).collect();
        let values = values.join(", ");
        let stmt = format!("{struct_name}_id AS (INSERT INTO \"{struct_name}\" ({columns}) VALUES ({values}) RETURNING id)");
        stmts.push(vec![stmt]);
    }
    stmts.into_iter().flatten().collect()
}

fn tok_to_string(tok: &Token) -> String {
    match tok {
        Token::U32(x) => format!("{x}"),
        Token::U64(x) => format!("{x}"),
        _ => unimplemented!("{tok:?}"),
    }
}

fn decl_fields(decl: &TypeDeclaration) -> Vec<TypeDeclaration> {
    let mut result = vec![];
    for field in decl.components.as_ref().unwrap() {
        let field_decl = crate::abi::type_declaration(field.type_id);
        result.push(field_decl)
    }
    result
}

fn is_struct(decl: &TypeDeclaration) -> bool {
    decl.type_field.starts_with("struct")
}

fn has_nested_struct(decl: &TypeDeclaration) -> bool {
    for field_decl in decl_fields(decl) {
        if is_struct(&field_decl) {
            return true;
        }
    }
    false
}
