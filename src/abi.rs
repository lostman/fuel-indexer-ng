use anyhow::Context;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, Read};

use fuel_abi_types::abi::program::{ProgramABI, TypeApplication, TypeDeclaration};
use fuels::types::param_types::ParamType;

#[derive(Debug, Clone)]
pub struct ABI {
    // map(type name => type id)
    pub type_ids: BTreeMap<String, usize>,
    // map(type id => param type)
    pub param_types: BTreeMap<usize, ParamType>,
    // map(type id => type declaration) from the "types" section of the ABI
    pub types: BTreeMap<usize, TypeDeclaration>,
    // map(logged type id => type id)
    pub logged_types: BTreeMap<usize, usize>,
}

pub fn print_abi(abi: &ABI) {
    println!(">> PARAM_TYPES");
    println!("{:#?}", abi.param_types);

    println!(">> TYPE_ID_MAP");
    println!("{:#?}", abi.type_ids);

    println!(">> TYPE_MAP");
    println!("{:#?}", abi.types);
}

impl ABI {
    pub fn param_type(&self, type_id: usize) -> ParamType {
        self.param_types.get(&type_id).unwrap().clone()
    }

    pub fn type_declaration(&self, type_id: usize) -> TypeDeclaration {
        self.types.get(&type_id).unwrap().clone()
    }

    pub fn type_id(&self, type_name: &str) -> usize {
        *self.type_ids.get(type_name).expect(&format!(
            "Unable to get type_id for '{type_name}' {:#?}",
            self.types
        ))
    }
}

pub fn parse_abi(script_abi_path: &str) -> anyhow::Result<ABI> {
    // Open the JSON file
    let file = std::fs::File::open(script_abi_path).context(script_abi_path.to_string())?;
    let mut reader = BufReader::new(file);

    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;

    let program_abi: ProgramABI = serde_json::from_str(&buf)?;
    println!(">> ABI:\n{program_abi:#?}");

    let type_lookup = program_abi
        .types
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, a_type)| (i, a_type))
        .collect::<HashMap<_, _>>();

    let json: serde_json::Value = serde_json::from_str(&buf)?;

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

        if !decl.type_field.starts_with("generic")
            && !decl.type_field.starts_with("raw")
            && !decl.type_field.starts_with("struct RawVec")
            && !decl.type_field.starts_with("struct Vec")
            && !decl.type_field.starts_with("enum Option")
        {
            let param_type = ParamType::try_from_type_application(&type_application, &type_lookup)
                .expect(&format!(
                    "Couldn't construct ParamType for {type_application:#?}"
                ));
            param_types.insert(type_id, param_type);
        }
        types.insert(type_id, decl.clone());
    }
    println!("FOOOOO");

    // 3. map(type name => type id)
    let mut type_ids = BTreeMap::new();
    for lt in json.get("types").unwrap().as_array().unwrap() {
        let type_name = lt.get("type").unwrap().as_str().unwrap();
        let type_id = lt.get("typeId").unwrap().as_u64().unwrap() as usize;
        type_ids.insert(type_name.to_string(), type_id);
    }

    let mut logged_types = BTreeMap::new();
    for lt in json.get("loggedTypes").unwrap().as_array().unwrap() {
        let log_id = lt.get("logId").unwrap().as_u64().unwrap() as usize;
        let type_id = lt
            .get("loggedType")
            .unwrap()
            .get("type")
            .unwrap()
            .as_u64()
            .unwrap() as usize;
        logged_types.insert(log_id, type_id);
    }

    let abi = ABI {
        types,
        type_ids,
        param_types,
        logged_types,
    };

    Ok(abi)
}
