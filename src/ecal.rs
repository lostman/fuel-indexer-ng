use std::collections::{HashMap, HashSet};
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

use crate::extensions::*;

#[derive(Debug, Clone)]
pub struct MyEcal {
    pub abi: crate::ABI,
    pub db_pool: Pool<Postgres>,
}

impl EcalHandler for MyEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        ra: RegId,
        rb: RegId,
        rc: RegId,
        rd: RegId,
    ) -> SimpleResult<()> {
        let a = vm.registers()[ra];
        println!("CALLING ECAL {a}");
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

        let param_type = vm.ecal_state_mut().abi.param_type(type_id);
        let tokens = ABIDecoder::new(DecoderConfig::default())
            .decode(&param_type, data.as_ref())
            .unwrap();
        println!(">> SAVE_ANY_TOKENS\n{tokens:#?}");
        // let stmt = save_any(&vm.ecal_state().abi, type_id, tokens);
        let stmt = SaveStmtBuilder::new(vm.ecal_state().abi.clone()).generate_stmt(type_id, tokens);
        println!(">> SAVE_STMT\n{stmt}");
        let rows_affected =
            futures::executor::block_on(sqlx::query(&stmt).execute(&vm.ecal_state().db_pool))
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

        let struct_name = vm
            .ecal_state_mut()
            .abi
            .type_declaration(type_id as usize)
            .type_field
            .strip_prefix("struct ")
            .unwrap()
            .to_string();
        let struct_token = load_any(
            &vm.ecal_state().db_pool,
            &vm.ecal_state().abi,
            struct_name,
            type_id as usize,
        );
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

        let param_type = vm.ecal_state_mut().abi.param_type(type_id);
        let tokens = ABIDecoder::new(DecoderConfig::default())
            .decode(&param_type, data.as_ref())
            .expect(&format!("{param_type:#?}"));
        // println!("> print_any = {tokens:?}");
        let result = pretty_print(&vm.ecal_state().abi, type_id, tokens);
        println!("> PRINT_ANY:");
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

        let type_id = vm.ecal_state_mut().abi.type_id(&type_name);

        vm.registers_mut()[rb] = type_id as u64;

        Ok(())
    }
}

// Given a type id and encoded data, it pretty-prints the data.
fn pretty_print(abi: &crate::ABI, type_id: usize, tok: Token) -> String {
    fn pretty_print_inner(
        abi: &crate::ABI,
        indent: usize,
        decl: TypeDeclaration,
        // For processing Option<T> types, need to pass the TypeDeclaration for T down.
        inner_decl: Option<TypeDeclaration>,
        tok: Token,
    ) -> String {
        match tok {
            Token::Unit => "()".to_string(),
            Token::U64(x) => format!("{}", x),
            Token::U32(x) => format!("{}", x),
            Token::U16(x) => format!("{}", x),
            Token::U8(x) => format!("{}", x),
            Token::Struct(fields) => {
                let indent = indent + 4;
                let comps = decl.components.unwrap();
                let mut result = vec![];
                for (i, field) in fields.into_iter().enumerate() {
                    let name: String = comps[i].name.clone();
                    let type_id: usize = comps[i].type_id;
                    let decl = abi.type_declaration(type_id);
                    result.push(
                        " ".repeat(indent)
                            + &name
                            + " = "
                            + &pretty_print_inner(abi, indent, decl, None, field),
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
                    let decl = abi.type_declaration(type_id);
                    result.push(
                        " ".repeat(indent) + &pretty_print_inner(abi, indent, decl, None, field),
                    )
                }
                "(\n".to_string() + &result.join(",\n") + "\n" + &" ".repeat(indent - 4) + ")"
            }
            Token::B256(bytes) => hex::encode(bytes),
            Token::U256(value) => hex::encode(Into::<[u8; 32]>::into(value)),
            Token::Bool(b) => format!("{b}"),
            Token::Array(elems) => {
                let inner_type = &decl.components.as_ref().unwrap()[0];
                // the inner_decl passed to the function; TODO: clean inner_decl
                // up somehow. need to know some types ahead but there has to be
                // a cleaner way
                assert!(inner_decl.is_none());
                let inner_decl = abi.types.get(&inner_type.type_id).unwrap();
                // We are simulating Vec<T> with [T; N], so we need a special case here
                let inner_inner_decl: Option<TypeDeclaration> =
                    if inner_decl.type_field.starts_with("enum") {
                        let inner_inner_type = &inner_type.type_arguments.as_ref().unwrap()[0];
                        Some(abi.types.get(&inner_inner_type.type_id).unwrap().clone())
                    } else {
                        None
                    };
                println!("ARRAY:\nINNER_DECL:\n{inner_decl:#?}\nINNER_INNER_DECL:\n{inner_inner_decl:#?}");
                let elems: Vec<String> = elems
                    .into_iter()
                    .map(|tok| {
                        pretty_print_inner(
                            abi,
                            indent,
                            inner_decl.clone(),
                            inner_inner_decl.clone(),
                            tok,
                        )
                    })
                    .collect::<Vec<String>>();
                "[".to_string() + &elems.join(", ") + "]"
            }
            Token::Enum(enum_selector) => {
                let (n, y, _) = *enum_selector;

                // e.g. Transaction::Mint(Mint) => Mint
                let component_type = decl.components.as_ref().unwrap()[n as usize].clone();

                // Sometimes we have an inner_decl, sometimes we need to look it up.
                // E.g. if we start with [Option<Transaction>; N], then we'll get to
                // Option<Transaction> with:
                // decl            , inner_decl      , inner_inner_decl
                // enum Option     , enum Transaction, enum Mint
                // enum Transaction, struct Mint     , None
                // struct Mint     , None            , None
                // If we start with plain Transaction
                // enum Transaction, None (this!)    , None
                let inner_decl =
                    inner_decl.or_else(|| abi.types.get(&component_type.type_id).cloned());

                let (variant, inner_inner_decl) = {
                    let component_type = decl.components.as_ref().unwrap()[n as usize].clone();
                    let variant = if decl.type_field == "enum Option" {
                        component_type.name.clone()
                    } else {
                        let type_name = decl.type_field.strip_prefix("enum ").unwrap().to_string();
                        type_name + "::" + &component_type.name
                    };
                    // println!("{variant} {type_id}", type_id = r#type.type_id);
                    let inner_inner_decl = {
                        if let Token::Enum(enum_selector) = y.clone() {
                            let (variant_number, _, _) = *enum_selector;
                            let target_type_id =
                                inner_decl.as_ref().unwrap().components.as_ref().unwrap()
                                    [variant_number as usize]
                                    .type_id;
                            abi.types.get(&target_type_id).cloned()
                        } else {
                            None
                        }
                    };
                    (variant, inner_inner_decl)
                };

                // println!("ENUM variant={variant}:\nDECL:\n{decl:#?}\nINNER_DECL:\n{inner_decl:#?}\nINNER_INNER_DECL:\n{inner_inner_decl:#?}\n{n}\n{y}\n{z:#?}");

                variant
                    + "("
                    + &pretty_print_inner(abi, indent, inner_decl.unwrap(), inner_inner_decl, y)
                    + ")"
            }
            _ => unimplemented!("pretty_print {tok:#?}"),
        }
    }
    let decl = abi.type_declaration(type_id);
    pretty_print_inner(abi, 0, decl, None, tok)
}

// WITH
//   MyStruct as (select one, two from mystruct),
//   MyOtherStruct as (select (value) from myotherstruct) (select * from MyStruct, MyOtherStruct);

use std::collections::{BTreeMap, VecDeque};

fn load_any(pool: &Pool<Postgres>, abi: &crate::ABI, struct_name: String, type_id: usize) -> Token {
    let mut context = HashMap::new();
    let (selects, joins, types) = load_any_rec(abi, HashSet::new(), &mut context, type_id as usize);
    let selects = selects.join(", ");
    let joins = joins.join(" ");

    let types: Vec<ParamType> = types.iter().map(|t| abi.param_type(*t)).collect();
    // TODO: until `load` accepts filter parameter, return a single value as a proof of concept
    let query_string =
        format!("SELECT {selects} FROM \"{struct_name}\" AS \"{struct_name}_0\" {joins} LIMIT 1");

    println!("LOAD_QUERY_STRING:\n{query_string}");

    let query = sqlx::query(&query_string);

    // TODO: handle empty result
    let row: sqlx::postgres::PgRow = futures::executor::block_on(query.fetch_one(pool)).unwrap();

    println!("RESULT ROW IS_EMPTY={}", row.is_empty());

    let mut tokens = VecDeque::new();
    for (index, t) in types.iter().enumerate() {
        let tok = match t {
            ParamType::U8 => Token::U8(row.get::<i8, usize>(index) as u8),
            ParamType::U16 => Token::U16(row.get::<i16, usize>(index) as u16),
            ParamType::U32 => Token::U32(row.get::<i32, usize>(index) as u32),
            ParamType::U64 => Token::U64(row.get::<i64, usize>(index) as u64),
            ParamType::B256 => Token::B256(
                hex::decode(row.get::<String, usize>(index))
                    .expect("decode hex to bytes")
                    .try_into()
                    .expect("convert bytes to [u8;32]"),
            ),

            ParamType::Bool => Token::Bool(row.get::<bool, usize>(index)),
            ParamType::U256 => {
                let x: Vec<u8> = hex::decode(row.get::<String, usize>(index))
                    .expect("decode hex to bytes")
                    .into();
                let y: [u8; 32] = x.try_into().unwrap();
                Token::U256(y.into())
            }
            _ => unimplemented!("{t:#?}"),
        };
        tokens.push_back(tok);
    }

    let decl = abi.type_declaration(type_id as usize);

    fn convert(
        abi: &crate::ABI,
        index: &mut usize,
        row: &sqlx::postgres::PgRow,
        decl: TypeDeclaration,
        params: &mut VecDeque<ParamType>,
    ) -> Token {
        // println!("CONVERT: {index} {decl:#?} {params:#?}");
        if decl.is_struct() && !decl.is_u256() {
            let mut target_value = vec![];
            for field in decl.components.unwrap().iter() {
                let field_decl = abi.type_declaration(field.type_id);
                let field_tokens = convert(abi, index, row, field_decl, params);
                target_value.push(field_tokens)
            }
            Token::Struct(target_value)
        } else {
            let field_token = match params.pop_front().unwrap() {
                ParamType::U32 => Token::U32(row.get::<i32, usize>(*index) as u32),
                ParamType::U64 => Token::U64(row.get::<i64, usize>(*index) as u64),
                ParamType::B256 => Token::B256(
                    hex::decode(row.get::<String, usize>(*index))
                        .expect("decode hex to bytes")
                        .try_into()
                        .expect("convert bytes to [u8;32]"),
                ),
                ParamType::U256 => {
                    let x = row.get::<String, usize>(*index);
                    let y = hex::decode(&x).expect("decode hex to bytes");
                    let z = TryInto::<[u8; 32]>::try_into(y).unwrap();
                    Token::U256(z.into())
                }
                ParamType::Bool => Token::Bool(row.get::<bool, usize>(*index)),
                _ => unimplemented!(),
            };
            *index += 1;
            field_token
        }
    }

    convert(abi, &mut 0, &row, decl, &mut types.into())
}

fn load_any_rec(
    abi: &crate::ABI,
    mut unique_joins: HashSet<String>,
    context: &mut HashMap<String, usize>,
    type_id: usize,
) -> (Vec<String>, Vec<String>, Vec<usize>) {
    println!("load_any_rec unique_joins={unique_joins:#?}");
    let decl = abi.type_declaration(type_id);

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
        let field_decl = abi.type_declaration(field.type_id);
        if field_decl.is_struct() && !field_decl.is_u256() {
            // println!("FOO");
            let field_struct_name = field_decl.type_field.strip_prefix("struct ").unwrap();
            let i = context
                .entry(field_struct_name.to_string())
                .and_modify(|x| *x += 1)
                .or_insert(0);

            let field_struct_alias = format!("{field_struct_name}_{i}");
            println!("load_any field_struct_alias={field_struct_alias}");
            let j = context.entry(struct_name.to_string()).or_insert(0);
            if unique_joins.insert(field_struct_alias.clone()) {
                let stmt = format!("LEFT JOIN \"{field_struct_name}\" AS \"{field_struct_alias}\" ON \"{struct_name}_{j}\".\"{field_name}Id\" = \"{field_struct_alias}\".id", field_name = field.name);
                joins.push(stmt);
            }

            let (nested_selects, nested_joins, nested_types) =
                load_any_rec(abi, unique_joins.clone(), context, field.type_id);
            println!("NESTED:\n{nested_selects:#?}\n{nested_joins:#?}");

            selects.extend(nested_selects);
            joins.extend(nested_joins);
            types.extend(nested_types);
        } else {
            println!("BAR");
            let i = context.get(&struct_name).unwrap_or(&0);
            let stmt = format!(
                "\"{struct_name}_{i}\".\"{field_name}\"",
                field_name = field.name
            );
            println!("load_any_rec select={stmt}");
            selects.push(stmt);
            types.push(field.type_id);
        }
    }

    (selects, joins, types)
}

struct SaveStmtBuilder {
    unique_stmts: HashSet<String>,
    stmts: Vec<String>,
    abi: crate::ABI,
}

impl SaveStmtBuilder {
    pub fn new(abi: crate::ABI) -> Self {
        Self {
            abi,
            stmts: vec![],
            unique_stmts: HashSet::new(),
        }
    }

    pub fn generate_stmt(&mut self, type_id: usize, target_value: Token) -> String {
        println!("GENERATE_STMT {type_id} {target_value:#?}");
        self.save_value(type_id, target_value);
        let stmts = self.stmts.join(", ");
        format!("WITH {stmts} (SELECT 1 AS placeholder_column_name)")
    }

    fn save_value(&mut self, type_id: usize, target_value: Token) {
        let target_decl = self.abi.type_declaration(type_id);

        println!(
            ">> SAVE_VALUE type_id={type_id} type={} tokens={target_value:#?}",
            target_decl.type_field
        );

        if target_value == Token::Unit {
            return;
        };

        if target_decl.is_option() {
            println!(">> SAVE_VALUE OPTION");
            // Component [0] is None, component [1] is Some.
            let elt_type = target_decl.components.as_ref().unwrap().clone()[1].clone();
            let elt = target_value.as_enum().1;
            println!(">> SAVING ELT type={elt_type:#?} elt={elt:#?}");
            if !(target_decl.type_field == "()") {
                self.save_value(elt_type.type_id, elt.clone());
            }
            return;
        }

        if target_decl.is_array() {
            println!(">> SAVE_VALUE ARRAY");
            let elt_type = target_decl.components.as_ref().unwrap().clone()[0].clone();
            for elt in target_value.as_array() {
                println!(">> SAVING ELT type={elt_type:#?} elt={elt:#?}");
                let type_id = elt_type.type_arguments.as_ref().unwrap()[0].type_id;
                self.save_value(type_id, elt.clone())
            }
            return;
        }

        let toks = if target_decl.is_struct() {
            println!("TOKS");
            for t in target_value.as_struct().clone().iter() {
                println!("\t{t:?}");
            }
            target_value.as_struct().clone()
        } else if target_decl.is_enum() {
            println!("{target_decl:#?}");
            println!("TOKS1");
            vec![target_value.as_enum().1]
        } else if target_decl.type_field == "()" {
            vec![]
        } else {
            panic!("{target_decl:#?}")
        };

        let mut columns: Vec<String> = target_decl
            .components
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|field| {
                let decl = self.abi.type_declaration(field.type_id);
                if decl.is_array() {
                    None
                } else if decl.is_struct() || decl.is_enum() && !decl.is_u256() {
                    Some(format!("\"{}Id\"", field.name))
                } else {
                    Some(format!("{}", field.name))
                }
            })
            .collect();
        if target_decl.has_nested_struct(&self.abi)
            || target_decl.has_nested_enum(&self.abi)
            || target_decl.has_nested_array(&self.abi)
        {
            let mut selects: Vec<String> = vec![];
            let mut sources: Vec<String> = vec![];
            let mut wheres = vec![];
            let inner_types = if target_decl.is_enum() {
                let n = target_value.as_enum().0;
                let k = target_decl.components.as_ref().unwrap()[n as usize].clone();
                // An enum is like a one-element Array, or a one-field struct.
                vec![k]
            } else if target_decl.is_array() {
                panic!("")
            } else {
                target_decl.components.as_ref().unwrap().clone()
            };
            for (i, field) in inner_types.iter().enumerate() {
                let field_decl = self.abi.type_declaration(field.type_id);
                println!(
                    "FIELD {name} DECL {i}/{n} decl={field_decl:#?}",
                    n = inner_types.len(),
                    i = i + 1,
                    name = field.name
                );
                let field_name = if field_decl.is_struct() || field_decl.is_enum() {
                    format!("{}Id", field.name)
                } else {
                    field.name.clone()
                };

                //
                // UNIT
                //
                if field_decl.type_field == "()" {
                    println!("UNIT SKIP")
                //
                // U256
                //
                } else if field_decl.is_u256() {
                    selects.push(tok_to_string(&toks[i]));
                //
                // ARRAY
                //
                } else if field_decl.is_array() {
                    let t = &toks[i];
                    let arr_type = field_decl.components.as_ref().unwrap().clone()[0].clone();
                    println!("ARR TYPE: {arr_type:#?}");
                    let elt_type = arr_type.type_arguments.as_ref().unwrap()[0].clone();
                    println!("ELT TYPE: {elt_type:#?}");
                    for elt in t.as_array() {
                        self.save_value(elt_type.type_id, elt.as_enum().1)
                    }

                    // Arrays have foreign keys pointing back.

                    // TODO: For example Block { transactions:
                    // [Option<Transaction>; 7]} After we've saved the elements
                    // of the array, we need to get the ID's and add an entry to
                    // the Trasnactions column
                    continue;
                //
                // ENUM
                //
                } else if field_decl.is_enum() {
                    let field_struct_name = field_decl.struct_or_enum_name().unwrap();

                    println!(
                        "BLARG OUTER:{target_decl:#?}\nFIELD {name} {field_decl:#?}",
                        name = field.name
                    );
                    if target_decl.is_array() {
                        println!("ARRAY SKIP");
                    } else {
                        self.save_value(
                            field_decl.type_id,
                            if field_decl.is_struct() {
                                println!("ONE");
                                toks[i].clone()
                            } else if field_decl.is_enum() {
                                println!("TWO {}", toks[i].clone());
                                toks[i].clone()
                            } else {
                                panic!("BLARG")
                            },
                        );
                    }

                    let field_struct_hash = hash_tokens(&vec![toks[i].as_enum().1]);

                    for variant in target_decl.components.as_ref().unwrap() {
                        let variant_decl = self.abi.types.get(&variant.type_id).unwrap();
                        if variant_decl.is_array() {
                            continue;
                        }
                        println!("VARIANT:\n{variant:#?}");
                        if target_decl.is_enum() && variant.name != field.name {
                            // NULLs for the values of other variants
                            selects.push(format!("NULL as {}Id", variant.name));
                            println!("SELECTS 5 {:?}", selects.last());
                        } else if target_decl.is_struct() {
                            // Id for the value of active variant
                            selects.push(format!(
                                "{field_struct_name}_id_{field_struct_hash}.id AS {field_name}"
                            ));
                            println!(
                                "SELECTS 4 {:?}\n{target_decl:#?}\n{variant_decl:#?}",
                                selects.last()
                            );
                        }
                    }

                    let source = format!("{field_struct_name}_id_{field_struct_hash}");

                    if !sources.contains(&source) {
                        sources.push(format!("{field_struct_name}_id_{field_struct_hash}"));
                    }

                    wheres.push(format!(
                            "\"{field_name}\" = (SELECT id FROM {field_struct_name}_id_{field_struct_hash})"
                        ));
                //
                // STRUCT
                //
                } else if field_decl.is_struct() {
                    let field_struct_name = field_decl.struct_or_enum_name().unwrap();
                    self.save_value(
                        field_decl.type_id,
                        if field_decl.is_struct() {
                            toks[i].clone()
                        } else {
                            toks[0].clone()
                        },
                    );

                    let field_struct_hash = hash_tokens(&toks[i].as_struct());

                    if target_decl.is_struct() {
                        selects.push(format!(
                            "{field_struct_name}_id_{field_struct_hash}.id AS {field_name}"
                        ));
                        println!("SELECTS 1 {:?}", selects.last());
                    } else if target_decl.is_enum() {
                        for variant in target_decl.components.as_ref().unwrap() {
                            println!("VARIANT 2:\n{variant:#?}");
                            if target_decl.is_enum() && variant.name != field.name {
                                // NULLs for the values of other variants
                                selects.push(format!("NULL as {}Id", variant.name));
                                println!("SELECTS 2 {:?}", selects.last());
                            } else {
                                // Id for the value of active variant
                                println!("SELECTS 3");
                                selects.push(format!(
                                    "{field_struct_name}_id_{field_struct_hash}.id AS {field_name}"
                                ));
                                println!("SELECTS 3 {:?}", selects.last());
                            }
                        }
                    }

                    let source = format!("{field_struct_name}_id_{field_struct_hash}");

                    if !sources.contains(&source) {
                        sources.push(format!("{field_struct_name}_id_{field_struct_hash}"));
                    }

                    wheres.push(format!(
                            "\"{field_name}\" = (SELECT id FROM {field_struct_name}_id_{field_struct_hash})"
                        ));
                //
                // OTHER
                //
                } else {
                    let tok = toks[i].clone();
                    if tok.is_array() {
                        // let elt_type = target_decl.components.as_ref().unwrap().clone()[0].clone();
                        // for elt in tok.as_array() {
                        //     self.save_value(field_decl.type_id, elt.clone())
                        // }
                    } else {
                        selects.push(tok_to_string(&tok));
                        wheres.push(format!("\"{field_name}\" = {}", tok_to_string(&tok)));
                    }
                }
            }
            let selects = selects.join(", ");
            let sources = sources.join(", ");
            let wheres = wheres.join(" AND ");
            let hash = hash_tokens(&toks);

            let struct_name = target_decl.struct_or_enum_name().unwrap();
            let stmt = format!("{struct_name}_new_row_{hash} AS (INSERT INTO \"{struct_name}\" ({columns}) (SELECT {selects} FROM {sources} WHERE NOT EXISTS (SELECT 1 FROM \"{struct_name}\" WHERE {wheres})) RETURNING id)", columns = columns.join(", "));
            self.push_stmt(stmt);

            let stmt = format!("{struct_name}_id_{hash} AS (SELECT id from {struct_name}_new_row_{hash} UNION ALL SELECT id from \"{struct_name}\" WHERE {wheres} LIMIT 1)");
            self.push_stmt(stmt);
        // No nested struct, enum, or array
        } else {
            let mut where_clause = vec![];
            let mut values: Vec<String> = vec![];
            for (i, t) in toks.iter().enumerate() {
                let col = columns[i].clone();

                // if t.is_enum() {
                //     let (n, inner, _) = t.as_enum();
                //     where_clause.push(format!("\"{}Variant\" = {}", col, n));
                //     where_clause.push(format!("{}Id = {}", col, 9876));

                //     columns[i] = format!("{col}Id");
                //     columns.insert(i, format!("\"{col}Variant\""));

                //     values.push("3".to_string());
                //     values.push(n.to_string());
                // } else if !t.is_array() && !t.is_struct() {
                where_clause.push(format!("{} = {}", col, tok_to_string(t)));
                values.push(tok_to_string(t));
                // }
            }
            let where_clause = where_clause.join(" AND ");

            let hash = hash_tokens(&toks);

            let struct_name = target_decl.struct_or_enum_name().unwrap();
            let stmt = format!("{struct_name}_new_row_{hash} AS (INSERT INTO \"{struct_name}\" ({columns}) SELECT {values} WHERE NOT EXISTS (SELECT 1 FROM \"{struct_name}\" WHERE {where_clause}) RETURNING id)", columns = columns.join(", "), values = values.join(", "));
            self.push_stmt(stmt);

            let stmt = format!("{struct_name}_id_{hash} AS (SELECT id from {struct_name}_new_row_{hash} UNION ALL SELECT id from \"{struct_name}\" WHERE {where_clause} LIMIT 1)");
            self.push_stmt(stmt);
        }
    }

    fn push_stmt(&mut self, s: String) {
        if self.unique_stmts.insert(s.clone()) {
            self.stmts.push(s);
        }
    }
}

fn tok_to_string(tok: &Token) -> String {
    match tok {
        Token::U8(x) => format!("{x}"),
        Token::U16(x) => format!("{x}"),
        Token::U32(x) => format!("{x}"),
        Token::U64(x) => format!("{x}"),
        Token::Bool(b) => format!("{b}"),
        Token::B256(bytes) => format!("\'{}\'", hex::encode(bytes)),
        Token::U256(value) => {
            let x = Into::<[u8; 32]>::into(*value);
            format!("\'{}\'", hex::encode(x))
        }
        // Token::Array(elems) => {
        //     format!(
        //         "[{}]",
        //         elems
        //             .iter()
        //             .map(tok_to_string)
        //             .collect::<Vec<String>>()
        //             .join(", ")
        //     )
        // }
        // Token::Enum(enum_selector) => {
        //     let (_, tok, _) = *enum_selector.to_owned();
        //     format!("ZZZ({})", tok_to_string(&tok))
        // }
        // Token::Struct(fields) => "STRUCT".to_string(),
        Token::Unit => "()".to_string(),
        _ => unimplemented!("{tok:?}"),
        // _ => "ZZZ".to_string(),
    }
}

// TODO: derive Hash for Token instead.
fn hash_tokens(tokens: &Vec<Token>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let s: String = format!("{tokens:#?}");
    s.hash(&mut hasher);
    hasher.finish()
}
