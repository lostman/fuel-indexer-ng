use anyhow::Context;
use lazy_static::lazy_static;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, Read};
use std::sync::Mutex;

use fuel_abi_types::abi::program::{ProgramABI, TypeApplication, TypeDeclaration};
use fuels::types::param_types::ParamType;

mod sql {
    pub use sqlparser::ast::helpers::stmt_create_table::CreateTableBuilder;
    pub use sqlparser::ast::{ColumnDef, DataType, Ident, ObjectName, Statement};
}

lazy_static! {
    // map(type name => type id)
    pub static ref TYPE_IDS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    // map(type id => param type)
    pub static ref PARAM_TYPES: Mutex<BTreeMap<usize, ParamType>> = Mutex::new(BTreeMap::new());
    // "types" section of the ABI
    pub static ref TYPES: Mutex<BTreeMap<usize, TypeDeclaration>> = Mutex::new(BTreeMap::new());
    static ref ABI_REF: Mutex<Option<ABI>> = Mutex::new(None);
}

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

pub fn param_type(type_id: usize) -> ParamType {
    crate::abi::ABI_REF
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .param_types
        .get(&type_id)
        .unwrap()
        .clone()
}

pub fn type_declaration(type_id: usize) -> TypeDeclaration {
    crate::abi::ABI_REF
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .types
        .get(&type_id)
        .unwrap()
        .clone()
}

pub fn type_id(type_name: &str) -> usize {
    *crate::abi::ABI_REF
        .lock()
        .unwrap()
        .as_ref()
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
        param_types.insert(type_id, param_type);
        types.insert(type_id, decl.clone());
    }

    println!("> Param Types");
    println!("{:#?}", param_types);

    // 3. map(type name => type id)
    let mut type_ids = BTreeMap::new();
    for lt in json.get("types").unwrap().as_array().unwrap() {
        let type_name = lt.get("type").unwrap().as_str().unwrap();
        let type_id = lt.get("typeId").unwrap().as_u64().unwrap() as usize;
        type_ids.insert(type_name.to_string(), type_id);
    }

    println!("> Type ID Map");
    println!("{:#?}", type_ids);

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
    *ABI_REF.lock().unwrap() = Some(abi);
}

pub struct SchemaConstructor {
    builders: Vec<sql::CreateTableBuilder>,
}

impl SchemaConstructor {
    pub fn new() -> Self {
        Self { builders: vec![] }
    }

    pub fn statements(self) -> Vec<sql::Statement> {
        let mut result = vec![];
        for b in self.builders {
            let stmt = b.build();
            result.push(stmt);
        }
        result
    }

    pub fn process_program_abi(&mut self, abi: &ABI) {
        for decl in abi.types.values() {
            if let Some(struct_name) = decl.type_field.strip_prefix("struct ") {
                self.process_struct(&abi, struct_name, decl.components.as_ref().unwrap())
            }
        }
    }

    fn process_struct(
        &mut self,
        abi: &ABI,
        struct_name: &str,
        struct_fields: &Vec<TypeApplication>,
    ) {
        let type_lookup = HashMap::from_iter(abi.types.clone());
        let columns: Vec<sql::ColumnDef> = struct_fields
            .iter()
            .map(|type_application| {
                let param_type =
                    ParamType::try_from_type_application(&type_application, &type_lookup).unwrap();
                Self::process_param_type(&type_application.name, param_type)
            })
            .flatten()
            .collect();

        let table_name = sql::ObjectName(vec![sql::Ident::new(struct_name)]);
        let builder = sql::CreateTableBuilder::new(table_name)
            .if_not_exists(true)
            .columns(columns);

        self.builders.push(builder);
    }

    fn process_param_type(name: &str, param_type: ParamType) -> Vec<sql::ColumnDef> {
        match param_type {
            ParamType::U64 => Self::one_column(name, sql::DataType::UnsignedBigInt(None)),
            ParamType::U32 => Self::one_column(name, sql::DataType::UnsignedInteger(None)),
            ParamType::Struct { .. } => Self::one_column(name, sql::DataType::BigInt(None)),
            ParamType::Tuple(elems) => {
                let mut columns = vec![];
                for (i, elem) in elems.iter().enumerate() {
                    let name = format!("{name}_{}", i);
                    let column = Self::process_param_type(&name, elem.clone());
                    columns.push(column);
                }
                columns.into_iter().flatten().collect()
            }
            _ => unimplemented!("TODO: {} {:?}", name, param_type),
        }
    }

    fn one_column(name: &str, data_type: sql::DataType) -> Vec<sql::ColumnDef> {
        vec![Self::column(name, data_type)]
    }

    fn column(name: &str, data_type: sql::DataType) -> sql::ColumnDef {
        sql::ColumnDef {
            name: sql::Ident::new(name),
            data_type,
            collation: None,
            options: vec![],
        }
    }
}
