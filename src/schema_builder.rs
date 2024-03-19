use case::CaseExt;
use sqlparser::ast::ColumnDef;
use std::collections::HashMap;

use fuel_abi_types::abi::program::{TypeApplication, TypeDeclaration};
use fuels::types::param_types::ParamType;

use crate::extensions::TypeDeclarationExt;

use crate::ABI;

mod sql {
    pub use sqlparser::ast::helpers::stmt_create_table::CreateTableBuilder;
    pub use sqlparser::ast::{
        ColumnDef, ColumnOption, ColumnOptionDef, DataType, ExactNumberInfo, Ident, ObjectName,
        Statement, TableConstraint,
    };

    pub fn quoted_ident(name: &str) -> Ident {
        Ident::new(format!("\"{name}\""))
    }

    // #[macro_export]
    // macro_rules! quoted_ident {
    //     ($($arg:tt)*) => {
    //         let name = format!($($arg)*);
    //         Ident::new(format!("\"{name}\""));
    //     };
    // }
}

pub struct SQLTableBuilder {
    builders: Vec<sql::CreateTableBuilder>,
    extra_builders: Vec<sql::CreateTableBuilder>,
    tables: std::collections::HashSet<String>,
    abi: ABI,
}

impl SQLTableBuilder {
    pub fn new(abi: ABI) -> Self {
        Self {
            builders: vec![],
            extra_builders: vec![],
            tables: std::collections::HashSet::new(),
            abi,
        }
    }

    pub fn statements(self) -> Vec<sql::Statement> {
        let mut result = vec![];
        for b in self.builders {
            let stmt = b.build();
            result.push(stmt);
        }
        for b in self.extra_builders {
            let stmt = b.build();
            result.push(stmt);
        }
        result
    }

    pub fn process_program_abi(&mut self, abi: &ABI) {
        let fuel_block_decl = self
            .abi
            .types
            .get(abi.type_ids.get("struct FuelBlock").unwrap())
            .unwrap();

        self.process_decl(fuel_block_decl.clone());

        for decl in abi.types.values() {
            println!("PROCESS DECL: {}", decl.type_field);
            self.process_decl(decl.clone())
        }
    }

    pub fn process_decl(&mut self, decl: TypeDeclaration) {
        if let Some(struct_name) = decl.type_field.strip_prefix("struct ") {
            self.process_struct(struct_name, decl.components.as_ref().unwrap())
        } else if let Some(enum_name) = decl.type_field.strip_prefix("enum ") {
            self.process_enum(enum_name)
        }
    }

    // ???
    // enum Transaction {
    //     Mint(Mint),
    //     Create(Create),
    // }
    fn process_enum(&mut self, enum_name: &str) {
        let type_id = self.abi.type_ids.get(&format!("enum {enum_name}")).unwrap();
        let decl = self.abi.types.get(type_id).unwrap().to_owned();

        // An enum with generic parameters (e.g. Option<T>). Skip it.
        if decl.type_parameters.is_some() {
            return;
        }

        // Conumns and tables for variants.
        let mut columns = vec![];

        columns.push(Self::pk_column());

        for c in decl.components.as_ref().unwrap() {
            let variant_decl = self.abi.types.get(&c.type_id).unwrap();
            self.process_decl(variant_decl.clone());

            columns.push(ColumnDef {
                name: sql::quoted_ident(&format!("{}Id", c.name)),
                data_type: sql::DataType::Int(None),
                collation: None,
                options: vec![],
            });
        }

        // Table for the enum.
        let table_name = sql::ObjectName(vec![sql::Ident::new(format!(
            "\"{}\"",
            decl.struct_or_enum_name().unwrap()
        ))]);

        let builder = sql::CreateTableBuilder::new(table_name)
            .if_not_exists(true)
            .columns(columns);
        self.builders.push(builder)
    }

    fn process_struct(&mut self, struct_name: &str, struct_fields: &Vec<TypeApplication>) {
        if self.tables.contains(struct_name) {
            println!("SKIPPING {struct_name}");
            return;
        };
        if struct_name == "U256" {
            return;
        }
        if struct_name == "RawVec" {
            return;
        }
        if struct_name == "Vec" {
            return;
        }
        let type_lookup = HashMap::from_iter(self.abi.types.clone());
        let mut columns: Vec<sql::ColumnDef> = struct_fields
            .iter()
            .map(|type_application| {
                let param_type =
                    ParamType::try_from_type_application(&type_application, &type_lookup)
                        .expect(&format!("{type_application:#?}"));
                self.process_param_type(
                    &type_application,
                    Some(struct_name.to_string()),
                    param_type,
                )
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

        if !self.tables.contains(&builder.name.to_string()) {
            self.tables.insert(builder.name.to_string());
            self.builders.push(builder);
        }
    }

    fn process_param_type(
        &mut self,
        type_application: &TypeApplication,
        struct_name: Option<String>,
        param_type: ParamType,
    ) -> Vec<sql::ColumnDef> {
        let name = &type_application.name;
        match param_type.clone() {
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
            // For a 64-bit unsigned integer, which ranges from 0 to
            // 18,446,744,073,709,551,615 (a 20-digit number), we need
            // Precision(20)
            ParamType::U64 => Self::one_column(
                name,
                sql::DataType::Numeric(sql::ExactNumberInfo::Precision(20)),
            ),
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
                for elem in elems.iter() {
                    let column = self.process_param_type(type_application, None, elem.clone());
                    columns.push(column);
                }
                columns.into_iter().flatten().collect()
            }
            ParamType::String => Self::one_column(name, sql::DataType::String(None)),
            ParamType::Bytes => Self::one_column(name, sql::DataType::Bytea),
            ParamType::Vector(_) => Self::one_column(name, sql::DataType::Bytea),
            ParamType::Enum { .. } => {
                let type_declaration = self.abi.types.get(&type_application.type_id).unwrap();
                // Option<_>
                if type_declaration.type_field == "enum Option" {
                    let x = &type_application.type_arguments.as_ref().unwrap()[0];
                    let y = self.abi.types.get(&x.type_id).unwrap();
                    // Option<Vec<_>>
                    if y.type_field == "struct Vec" {
                        let z = &x.type_arguments.as_ref().unwrap()[0];
                        let z2 = self.abi.types.get(&z.type_id).unwrap();
                        // Special case for Option<Vec<u8>>
                        if z2.type_field == "u8" {
                            Self::one_column(name, sql::DataType::Bytea)
                        } else {
                            unimplemented!()
                        }
                    } else if y.is_struct() {
                        Self::one_column(&format!("{name}Id"), sql::DataType::Integer(None))
                    } else {
                        unimplemented!("{name} {type_application:#?} {type_declaration:#?} {y:#?}")
                    }
                } else {
                    // panic!("{type_application:#?}{type_declaration:#?}\n{param_type:#?}")
                    vec![
                        Self::column(&format!("\"{name}Variant\""), sql::DataType::Int(None)),
                        Self::column(&format!("\"{name}Id\""), sql::DataType::BigInt(None)),
                    ]
                }
            }
            ParamType::Array(elem_type, _) => {
                // WHAT NEEDS TO HAPPEN HERE:
                // e.g. [Option<Transaction>; 10]
                // So... strip the Array<Option<>>, leaving Transaction
                // Create a table Transactions
                // Transactions.block_id: INT NOT NULL; FOREIGN KEY (block_id) REFERENCES Block.id;

                let decl = self
                    .abi
                    .types
                    .get(&type_application.type_id)
                    .unwrap()
                    .to_owned();

                let inner_type = decl.components.as_ref().unwrap()[0].clone();
                let inner_decl = self.abi.types.get(&inner_type.type_id).unwrap();
                if inner_decl.type_field == "enum Option" {
                    let table_name = sql::ObjectName(vec![sql::Ident::new(format!(
                        "\"{}\"",
                        name.as_str().to_capitalized()
                    ))]);

                    if !self.tables.contains(&table_name.to_string()) {
                        let columns = vec![
                            Self::column("id", sql::DataType::Int(None)),
                            Self::column(
                                &format!("{}_id", struct_name.as_ref().unwrap().to_snake()),
                                sql::DataType::Int(None),
                            ),
                        ];
                        let mut foreign_table_ident =
                            sql::Ident::new(struct_name.as_ref().unwrap().to_capitalized());
                        foreign_table_ident.quote_style = Some('"');
                        let constraints = vec![sql::TableConstraint::ForeignKey {
                            name: None, // sql::Ident::new("value"),
                            columns: vec![sql::Ident::new(&format!(
                                "{}_id",
                                struct_name.as_ref().unwrap().to_snake()
                            ))],
                            foreign_table: sql::ObjectName(vec![foreign_table_ident]),
                            referred_columns: vec![sql::Ident::new("id")],
                            on_delete: Some(sqlparser::ast::ReferentialAction::Cascade),
                            on_update: None,
                        }];

                        let builder = sql::CreateTableBuilder::new(table_name.clone())
                            .if_not_exists(true)
                            .columns(columns)
                            .constraints(constraints);

                        self.tables.insert(builder.name.to_string());

                        // `FuelBlock`` contains `transactions: Transaction`
                        // which is an enum field and so we need to create a
                        // table for it. `Transactins` table contains a FK
                        // constraint on `FuelBlock`, so `FuelBlock` create
                        // statement must be emitted first. So, `FuelBlock` goes
                        // to `builders` and `Transaction` to `extra_builders`
                        // which is converted to a statent later.
                        self.extra_builders.push(builder);
                    }
                }

                vec![]
                // TODO
                // Self::one_column(&format!("{name}Id"), sql::DataType::BigInt(None))
                // panic!("{:?}", r#type)
            }
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
