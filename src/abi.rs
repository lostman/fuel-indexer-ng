use anyhow::Context;
use lazy_static::lazy_static;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, Read};
use std::sync::Mutex;

use fuel_abi_types::abi::program::{ProgramABI, TypeApplication, TypeDeclaration};
use fuels::types::param_types::ParamType;

lazy_static! {
    // map(type name => type id)
    pub static ref TYPE_IDS: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());
    // map(type id => param type)
    pub static ref PARAM_TYPES: Mutex<BTreeMap<u64, ParamType>> = Mutex::new(BTreeMap::new());
    // "types" section of the ABI
    pub static ref TYPES: Mutex<BTreeMap<u64, TypeDeclaration>> = Mutex::new(BTreeMap::new());
}

pub fn param_type(type_id: u64) -> ParamType {
    crate::abi::PARAM_TYPES
        .lock()
        .unwrap()
        .get(&type_id)
        .unwrap()
        .clone()
}

pub fn type_declaration(type_id: u64) -> TypeDeclaration {
    crate::abi::TYPES
        .lock()
        .unwrap()
        .get(&type_id)
        .unwrap()
        .clone()
}

pub fn type_id(type_name: &str) -> u64 {
    *crate::abi::TYPE_IDS
        .lock()
        .unwrap()
        .get(type_name)
        .expect(&format!("{type_name}"))
}

pub fn parse_abi(script_abi_path: &str) -> anyhow::Result<()> {
    // Open the JSON file
    let file = std::fs::File::open(script_abi_path).context(script_abi_path.to_string())?;
    let mut reader = BufReader::new(file);

    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;

    let program_abi: ProgramABI = serde_json::from_str(&buf)?;
    println!("> ABI:{program_abi:#?}");

    let type_lookup = program_abi
        .types
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, a_type)| (i, a_type))
        .collect::<HashMap<_, _>>();

    let json: serde_json::Value = serde_json::from_str(&buf)?;

    let pretty_json = serde_json::to_string_pretty(&json)?;

    // Print the pretty-printed JSON
    println!("> ABI");
    println!("{pretty_json}");

    // 1. Store contents of "types" for generic struct processing
    let mut types = crate::abi::TYPES.lock().unwrap();

    // 2. map(type id => param type)
    let mut param_types = crate::abi::PARAM_TYPES.lock().unwrap();
    for (type_id, decl) in program_abi.types.iter().enumerate() {
        let type_application = TypeApplication {
            name: decl.type_field.clone(),
            type_id,
            type_arguments: decl.components.clone(),
        };
        let param_type = ParamType::try_from_type_application(&type_application, &type_lookup)?;
        param_types.insert(type_id as u64, param_type);
        types.insert(type_id as u64, decl.clone());
    }

    println!("> Param Types");
    println!("{:#?}", param_types);

    // 3. map(type name => type id)
    let mut type_map = crate::abi::TYPE_IDS.lock().unwrap();
    for lt in json.get("types").unwrap().as_array().unwrap() {
        let type_name = lt.get("type").unwrap().as_str().unwrap();
        let type_id = lt.get("typeId").unwrap().as_u64().unwrap();
        type_map.insert(type_name.to_string(), type_id);
    }

    println!("> Type ID Map");
    println!("{:#?}", type_map);

    Ok(())
}
