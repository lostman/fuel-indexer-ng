use fuel_abi_types::abi::program::TypeDeclaration;
use fuel_asm::RegId;
use fuel_vm::{
    error::SimpleResult,
    prelude::{Interpreter, MemoryRange},
};
use fuels::core::codec::ABIEncoder;
use fuels::types::param_types::ParamType;
use fuels::types::Token;

use sqlx::{Pool, Postgres, Row};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::extensions::*;

pub fn load<S, Tx>(vm: &mut Interpreter<S, Tx, super::MyEcal>, rb: RegId) -> SimpleResult<()> {
    let type_id = vm.registers()[rb];
    #[cfg(debug_assertions)]
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
