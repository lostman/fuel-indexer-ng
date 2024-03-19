use fuel_abi_types::abi::program::TypeDeclaration;
use fuel_asm::RegId;
use fuel_vm::{
    error::SimpleResult,
    prelude::{Interpreter, MemoryRange},
};
use fuels::core::codec::ABIDecoder;
use fuels::types::Token;

pub fn println<S, Tx>(vm: &mut Interpreter<S, Tx, super::MyEcal>, rb: RegId) -> SimpleResult<()> {
    let start = std::time::Instant::now();

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

    // println!("print_any_ecal type_id = {type_id}");

    let param_type = vm.ecal_state_mut().abi.param_type(type_id);
    let tokens = ABIDecoder::new(super::DECODER_CONFIG)
        .decode(&param_type, data.as_ref())
        .expect(&format!("{param_type:#?}"));
    // println!("> print_any = {tokens:?}");
    let result = pretty_print(&vm.ecal_state().abi, type_id, tokens);

    #[cfg(debug_assertions)]
    println!("> PRINT_ANY:");

    println!("{result}");

    let duration = start.elapsed();

    println!("ECAL::print execution time: {duration:?}");

    Ok(())
}

// Given a type id and encoded data, it pretty-prints the data.
pub fn pretty_print(abi: &crate::ABI, type_id: usize, tok: Token) -> String {
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

                #[cfg(debug_assertions)]
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
