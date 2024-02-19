use anyhow::Context;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, Read};

use fuel_abi_types::abi::program::{ProgramABI, TypeApplication, TypeDeclaration};
use fuels::types::param_types::ParamType;

mod sql {
    pub use sqlparser::ast::helpers::stmt_create_table::CreateTableBuilder;
    pub use sqlparser::ast::{
        ColumnDef, ColumnOption, ColumnOptionDef, DataType, Ident, ObjectName, Statement,
    };
}

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
        *self.type_ids.get(type_name).expect(&format!("{type_name}"))
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
        let param_type =
            ParamType::try_from_type_application(&type_application, &type_lookup).expect("1");
        param_types.insert(type_id, param_type);
        types.insert(type_id, decl.clone());
    }

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
        if struct_name == "U256" {
            return;
        }
        let type_lookup = HashMap::from_iter(abi.types.clone());
        let mut columns: Vec<sql::ColumnDef> = struct_fields
            .iter()
            .map(|type_application| {
                let param_type =
                    ParamType::try_from_type_application(&type_application, &type_lookup)
                        .expect("2");
                Self::process_param_type(&type_application.name, param_type)
            })
            .flatten()
            .collect();
        columns.push(Self::pk_column());
        // move 'id' column to the front
        columns.rotate_right(1);

        let table_name = sql::ObjectName(vec![sql::Ident::new(format!("\"{struct_name}\""))]);
        let builder = sql::CreateTableBuilder::new(table_name)
            .if_not_exists(true)
            .columns(columns);

        self.builders.push(builder);
    }

    fn process_param_type(name: &str, param_type: ParamType) -> Vec<sql::ColumnDef> {
        match param_type {
            ParamType::Bool => Self::one_column(name, sql::DataType::Boolean),
            // TODO: add constraints
            // CREATE TABLE my_table (
            //     id INTEGER,
            //     -- other columns
            //     CONSTRAINT positive_id CHECK (id >= 0)
            // );
            ParamType::U8 | ParamType::U16 | ParamType::U32 => {
                Self::one_column(name, sql::DataType::Integer(None))
            }
            ParamType::U64 => Self::one_column(name, sql::DataType::BigInt(None)),
            // hex-encoded
            ParamType::U128 => Self::one_column(name, sql::DataType::Text),
            // hex-encoded
            ParamType::B256 => Self::one_column(name, sql::DataType::Text),
            ParamType::U256 => Self::one_column(name, sql::DataType::Text),
            ParamType::Struct { .. } => {
                Self::one_column(&format!("{name}Id"), sql::DataType::BigInt(None))
            }
            ParamType::Tuple(elems) => {
                let mut columns = vec![];
                for (i, elem) in elems.iter().enumerate() {
                    let name = format!("{name}_{}", i);
                    let column = Self::process_param_type(&name, elem.clone());
                    columns.push(column);
                }
                columns.into_iter().flatten().collect()
            }
            ParamType::String => Self::one_column(name, sql::DataType::String(None)),
            ParamType::Bytes => Self::one_column(name, sql::DataType::Bytea),
            _ => unimplemented!("TODO: `{}: {:?}`", name, param_type),
        }
    }

    // pub enum ParamType {
    // x   U8,   x
    // x   U16,  x
    // x   U32,  x
    // x   U64,  x
    // x   U128, x
    // x   U256, x
    // x   Bool, x
    // x   B256, x
    //     Unit,
    //     Array(Box<ParamType>, usize),
    //     Vector(Box<ParamType>),
    //     StringSlice,
    //     StringArray(usize),
    //     Struct {
    //         fields: Vec<ParamType>,
    //         generics: Vec<ParamType>,
    //     },
    //     Enum {
    //         variants: EnumVariants,
    //         generics: Vec<ParamType>,
    //     },
    //     Tuple(Vec<ParamType>),
    //     RawSlice,
    // x   Bytes,
    // x   String,
    // }

    fn one_column(name: &str, data_type: sql::DataType) -> Vec<sql::ColumnDef> {
        vec![Self::column(&format!("\"{}\"", name), data_type)]
    }

    fn column(name: &str, data_type: sql::DataType) -> sql::ColumnDef {
        sql::ColumnDef {
            name: sql::Ident::new(name),
            data_type,
            collation: None,
            options: vec![],
        }
    }

    // id SERIAL PRIMARY KEY
    fn pk_column() -> sql::ColumnDef {
        sql::ColumnDef {
            name: sql::Ident::new("id"),
            data_type: sql::DataType::Custom(
                sqlparser::ast::ObjectName(vec!["SERIAL".into()]),
                vec![],
            ),
            collation: None,
            options: vec![sql::ColumnOptionDef {
                name: None,
                option: sql::ColumnOption::Unique { is_primary: true },
            }],
        }
    }
}
