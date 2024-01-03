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
    static ref ABI_REF: Mutex<ABI> = Mutex::new(ABI::new());
}

pub struct ABI {
    // map(type name => type id)
    pub type_ids: BTreeMap<String, u64>,
    // map(type id => param type)
    pub param_types: BTreeMap<u64, ParamType>,
    // map(type id => type declaration) from the "types" section of the ABI
    pub types: BTreeMap<u64, TypeDeclaration>,
    // map(logged type id => type id)
    pub logged_types: BTreeMap<u64, u64>,
}

impl ABI {
    fn new() -> Self {
        ABI {
            type_ids: BTreeMap::new(),
            param_types: BTreeMap::new(),
            types: BTreeMap::new(),
            logged_types: BTreeMap::new(),
        }
    }
}

pub fn param_type(type_id: u64) -> ParamType {
    crate::abi::ABI_REF
        .lock()
        .unwrap()
        .param_types
        .get(&type_id)
        .unwrap()
        .clone()
}

pub fn type_declaration(type_id: u64) -> TypeDeclaration {
    crate::abi::ABI_REF
        .lock()
        .unwrap()
        .types
        .get(&type_id)
        .unwrap()
        .clone()
}

pub fn type_id(type_name: &str) -> u64 {
    *crate::abi::ABI_REF
        .lock()
        .unwrap()
        .type_ids
        .get(type_name)
        .expect(&format!("{type_name}"))
}

pub fn parse_abi(script_abi_path: &str) -> anyhow::Result<ABI> {
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
    let mut types = BTreeMap::new();

    // 2. map(type id => param type)
    let mut param_types = BTreeMap::new();
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
    let mut type_ids = BTreeMap::new();
    for lt in json.get("types").unwrap().as_array().unwrap() {
        let type_name = lt.get("type").unwrap().as_str().unwrap();
        let type_id = lt.get("typeId").unwrap().as_u64().unwrap();
        type_ids.insert(type_name.to_string(), type_id);
    }

    println!("> Type ID Map");
    println!("{:#?}", type_ids);

    let mut logged_types = BTreeMap::new();
    for lt in json.get("loggedTypes").unwrap().as_array().unwrap() {
        let log_id = lt.get("logId").unwrap().as_u64().unwrap();
        let type_id = lt
            .get("loggedType")
            .unwrap()
            .get("type")
            .unwrap()
            .as_u64()
            .unwrap();
        logged_types.insert(log_id, type_id);
    }

    println!("> Type Map");
    println!("{:#?}", types);

    let abi = ABI {
        types,
        type_ids,
        param_types,
        logged_types,
    };

    Ok(abi)
}

pub fn set_ecal_abi(abi: ABI) {
    *ABI_REF.lock().unwrap() = abi;
}
