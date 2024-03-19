use fuel_asm::RegId;
use fuel_vm::{
    error::SimpleResult,
    prelude::{Interpreter, MemoryRange},
};
use fuels::core::codec::ABIDecoder;
use fuels::types::Token;

use std::collections::HashSet;

use crate::extensions::*;

pub fn save<S, Tx>(vm: &mut Interpreter<S, Tx, super::MyEcal>, rb: RegId) -> SimpleResult<()> {
    let start = std::time::Instant::now();

    #[cfg(debug_assertions)]
    println!(">> ECAL::save()");
    let (type_id, addr, size): (u64, u64, u64) = {
        let addr = vm.registers()[rb];
        let r = MemoryRange::new(addr, 3 * 8)?;
        let bytes: [u8; 3 * 8] = vm.memory()[r.usizes()].try_into().unwrap();
        fuels::core::codec::try_from_bytes(&bytes, super::DECODER_CONFIG).unwrap()
    };
    let type_id = type_id as usize;

    let data = {
        let r = MemoryRange::new(addr, size)?;
        let mut bytes = Vec::with_capacity(size as usize);
        bytes.extend_from_slice(&vm.memory()[r.usizes()]);
        bytes
    };

    let param_type = vm.ecal_state_mut().abi.param_type(type_id);
    let tokens = ABIDecoder::new(super::DECODER_CONFIG)
        .decode(&param_type, data.as_ref())
        .unwrap();
    // println!(">> SAVE_ANY_TOKENS\n{tokens:#?}");
    // let stmt = save_any(&vm.ecal_state().abi, type_id, tokens);
    let generate_start = std::time::Instant::now();
    let stmt = SaveStmtBuilder::new(vm.ecal_state().abi.clone()).generate_stmt(type_id, tokens);
    let generate_duration = generate_start.elapsed();

    #[cfg(debug_assertions)]
    println!(">> SAVE_STMT\n{stmt}");

    let exec_start = std::time::Instant::now();
    let rows_affected =
        futures::executor::block_on(sqlx::query(&stmt).execute(&vm.ecal_state().db_pool))
            .unwrap()
            .rows_affected();
    let exec_duration = exec_start.elapsed();

    let duration = start.elapsed();

    println!("ECAL::save: {duration:?}, stmt gen: {generate_duration:?}, stmt exec: {exec_duration:?}, rows affected {rows_affected}");

    Ok(())
}

// WITH
//   MyStruct as (select one, two from mystruct),
//   MyOtherStruct as (select (value) from myotherstruct) (select * from MyStruct, MyOtherStruct);

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
        #[cfg(debug_assertions)]
        println!("GENERATE_STMT {type_id} {target_value:#?}");
        self.save_value(type_id, target_value);
        let stmts = self.stmts.join(", ");
        // This will not return any rows, but it's a valid SQL statement.
        let noop = "SELECT 1 WHERE FALSE";
        // A valid SQL statement must follow the WITH clause
        format!("WITH {stmts} {noop}")
    }

    fn save_value(&mut self, type_id: usize, target_value: Token) {
        if target_value == Token::Unit {
            return;
        };

        let target_decl = self.abi.type_declaration(type_id);

        #[cfg(debug_assertions)]
        println!(
            ">> SAVE_VALUE type_id={type_id} type={} tokens={target_value:#?}",
            target_decl.type_field
        );

        if !(target_value.is_array() || target_value.is_enum() || target_value.is_struct()) {
            panic!("Expected array, enum, or struct, but got: {target_value:?}");
        }

        if target_decl.is_option() {
            #[cfg(debug_assertions)]
            println!(">> SAVE_VALUE OPTION");
            // Component [0] is None, component [1] is Some.
            let elt_type = target_decl.components.as_ref().unwrap().clone()[1].clone();
            let elt = target_value.as_enum().1;
            #[cfg(debug_assertions)]
            println!(">> SAVING ELT type={elt_type:#?} elt={elt:?}");
            // If `Some(v)`, save `v`, else, do nothing when `None`
            if !(target_decl.type_field == "()") {
                self.save_value(elt_type.type_id, elt.clone());
            }
            return;
        }

        if target_decl.is_array() {
            #[cfg(debug_assertions)]
            println!(">> SAVE_VALUE ARRAY");
            let elt_type = target_decl.components.as_ref().unwrap().clone()[0].clone();
            for elt in target_value.as_array() {
                #[cfg(debug_assertions)]
                println!(">> SAVING ELT type={elt_type:#?} elt={elt:?}");
                let type_id = elt_type.type_arguments.as_ref().unwrap()[0].type_id;
                self.save_value(type_id, elt.clone())
            }
            return;
        }

        let toks = if target_decl.is_struct() {
            // println!("TOKS");
            // for t in target_value.as_struct().clone().iter() {
            //     println!("\t{t:?}");
            // }
            target_value.as_struct().clone()
        } else if target_decl.is_enum() {
            // println!("{target_decl:#?}");
            // println!("TOKS1");
            vec![target_value.as_enum().1]
        } else if target_decl.type_field == "()" {
            vec![]
        } else {
            panic!("{target_decl:#?}\n{target_value:#?}")
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
                } else if (decl.is_struct() || decl.is_enum()) && !decl.is_u256() {
                    Some(format!("\"{}Id\"", field.name))
                } else {
                    Some(format!("\"{}\"", field.name))
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
                #[cfg(debug_assertions)]
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
                    #[cfg(debug_assertions)]
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

                    let arr_elt_decl = self.abi.types.get(&arr_type.type_id).unwrap();

                    #[cfg(debug_assertions)]
                    println!("ARR TYPE: {arr_type:#?}\nARR ELT DECL:\n{arr_elt_decl:#?}");

                    let elt_type = arr_type.type_arguments.as_ref().unwrap()[0].clone();
                    let elt_decl = self.abi.types.get(&elt_type.type_id).unwrap();

                    #[cfg(debug_assertions)]
                    println!("ELT TYPE: {elt_type:#?}\n{elt_decl:#?}");

                    // [Option<u8>; N] to simulate Vec<u8>
                    if arr_elt_decl.type_field == "enum Option" && !elt_decl.is_entity() {
                        // TODO: turn [Option<u8>; N] into [u8] and save it as hex string
                    } else {
                        for elt in t.as_array() {
                            self.save_value(elt_type.type_id, elt.as_enum().1)
                        }
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

                    #[cfg(debug_assertions)]
                    println!(
                        "BLARG OUTER:{target_decl:#?}\nFIELD {name} {field_decl:#?}",
                        name = field.name
                    );
                    if target_decl.is_array() {
                        #[cfg(debug_assertions)]
                        println!("ARRAY SKIP");
                    } else {
                        self.save_value(
                            field_decl.type_id,
                            if field_decl.is_struct() {
                                #[cfg(debug_assertions)]
                                println!("ONE");
                                toks[i].clone()
                            } else if field_decl.is_enum() {
                                #[cfg(debug_assertions)]
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
                        #[cfg(debug_assertions)]
                        println!("VARIANT:\n{variant:#?}");
                        if target_decl.is_enum() && variant.name != field.name {
                            // NULLs for the values of other variants
                            selects.push(format!("NULL as {}Id", variant.name));
                            #[cfg(debug_assertions)]
                            println!("SELECTS 5 {:?}", selects.last());
                        } else if target_decl.is_struct() {
                            // Id for the value of active variant
                            selects.push(format!(
                                "{field_struct_name}_id_{field_struct_hash}.id AS {field_name}"
                            ));
                            #[cfg(debug_assertions)]
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
                        #[cfg(debug_assertions)]
                        println!("SELECTS 1 {:?}", selects.last());
                    } else if target_decl.is_enum() {
                        for variant in target_decl.components.as_ref().unwrap() {
                            #[cfg(debug_assertions)]
                            println!("VARIANT 2:\n{variant:#?}");
                            if target_decl.is_enum() && variant.name != field.name {
                                // NULLs for the values of other variants
                                selects.push(format!("NULL as {}Id", variant.name));
                                #[cfg(debug_assertions)]
                                println!("SELECTS 2 {:?}", selects.last());
                            } else {
                                // Id for the value of active variant
                                #[cfg(debug_assertions)]
                                println!("SELECTS 3");

                                selects.push(format!(
                                    "{field_struct_name}_id_{field_struct_hash}.id AS {field_name}"
                                ));

                                #[cfg(debug_assertions)]
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
            let columns = if columns.is_empty() {
                "".to_string()
            } else {
                format!("({})", columns.join(", "))
            };
            let wheres = if wheres.is_empty() {
                "".to_string()
            } else {
                "WHERE ".to_string() + &wheres
            };
            let sources = if sources.is_empty() {
                "".to_string()
            } else {
                "FROM ".to_string() + &sources
            };
            let stmt = format!("{struct_name}_new_row_{hash} AS (INSERT INTO \"{struct_name}\" {columns} (SELECT {selects} {sources} WHERE NOT EXISTS (SELECT 1 FROM \"{struct_name}\" {wheres})) RETURNING id)");
            self.push_stmt(stmt);

            let stmt = format!("{struct_name}_id_{hash} AS (SELECT id from {struct_name}_new_row_{hash} UNION ALL SELECT id from \"{struct_name}\" {wheres} LIMIT 1)");
            self.push_stmt(stmt);
        // No nested struct, enum, or array
        } else {
            #[cfg(debug_assertions)]
            println!("PLAIN DATA");
            let mut where_clause = vec![];
            let mut values: Vec<String> = vec![];
            for (i, t) in toks.iter().enumerate() {
                // TODO: plain enum without nested structs, enums, or arrays.
                // HOW TO REPRESENT IT IN THE DB? CUSTOM ENUM TYPE?
                if target_decl.is_enum() {
                    let (n, b, c) = target_value.as_enum();

                    columns = target_decl
                        .components
                        .as_ref()
                        .unwrap()
                        .iter()
                        .enumerate()
                        .filter_map(|(m, _)| {
                            if n == (m as u64) {
                                Some("Variant".to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    // let mut columns: Vec<String> = target_decl
                    //     .components
                    //     .as_ref()
                    //     .unwrap()
                    //     .iter()
                    //     .filter_map(|field| {
                    //         let decl = self.abi.type_declaration(field.type_id);
                    //         if decl.is_array() {
                    //             None
                    //         } else if (decl.is_struct() || decl.is_enum()) && !decl.is_u256() {
                    //             Some(format!("\"{}Id\"", field.name))
                    //         } else {
                    //             Some(format!("\"{}\"", field.name))
                    //         }
                    //     })
                    //     .collect();

                    // TODO: not quite correct
                    // columns[i] = format!("{}Id", columns[i]);

                    // where_clause.push(format!("\"{}Variant\" = {}", col, n));
                    where_clause.push(format!("Variant = {}", columns[i]));
                    //values.push("3".to_string());
                    //values.push(n.to_string());
                } else {
                    #[cfg(debug_assertions)]
                    println!(
                        "FOOBAR {} {t:?}",
                        target_decl.struct_or_enum_name().unwrap()
                    );
                    where_clause.push(format!("{} = {}", columns[i], tok_to_string(t)));

                    values.push(tok_to_string(t));
                }
            }
            let where_clause = where_clause.join(" AND ");

            let hash = hash_tokens(&toks);

            let struct_name = target_decl.struct_or_enum_name().unwrap();

            let columns = if columns.is_empty() {
                "".to_string()
            } else {
                format!("({})", columns.join(", "))
            };
            let where_clause = if where_clause.is_empty() {
                "".to_string()
            } else {
                "WHERE ".to_string() + &where_clause
            };
            let values = if values.is_empty() {
                "SELECT".to_string()
            } else {
                "SELECT ".to_string() + &values.join(", ")
            };
            let stmt = format!("{struct_name}_new_row_{hash} AS (INSERT INTO \"{struct_name}\" {columns} {values} WHERE NOT EXISTS (SELECT 1 FROM \"{struct_name}\" {where_clause}) RETURNING id)");
            self.push_stmt(stmt);

            let stmt = format!("{struct_name}_id_{hash} AS (SELECT id from {struct_name}_new_row_{hash} UNION ALL SELECT id from \"{struct_name}\" {where_clause} LIMIT 1)");
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
