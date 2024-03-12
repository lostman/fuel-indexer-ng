use datamodel_renderer::datamodel::{Field, IdFieldDefinition, Model, Relation};
use datamodel_renderer::{self, Datamodel};

use fuel_abi_types::abi::program::{TypeApplication, TypeDeclaration};

use std::any::Any;
use std::collections::BTreeMap;

// Generate a Prisma schema from Fuel ABI
pub fn schema_from_abi(types: &BTreeMap<usize, TypeDeclaration>) -> String {
    println!(">> GENERATING PRISMA SCHEMA");
    let mut data_model = Datamodel::new();
    for TypeDeclaration {
        type_field,
        components,
        ..
    } in types.values()
    {
        if let Some(name) = type_field.strip_prefix("struct ") {
            if name == "U256" {
                continue;
            }
            println!(">> PROCESSING STRUCT: {name}");
            let mut model = Model::new(name.to_string());
            let mut id_field = Field::new("id", "Int");
            id_field.id(IdFieldDefinition::new());
            model.push_field(id_field);
            for TypeApplication { name, type_id, type_arguments } in components.as_ref().unwrap() {
                let field_decl = types
                    .get(&type_id)
                    .expect(&format!("TypeDeclaration for TypeApplication {name} with type_id={type_id}"));
                match field_decl.type_field.as_str() {
                    "u8" => model.push_field(Field::new(name, "Int")),
                    "u16" => model.push_field(Field::new(name, "Int")),
                    "u32" => model.push_field(Field::new(name, "Int")),
                    "u64" => model.push_field(Field::new(name, "Int")),
                    "b256" => model.push_field(Field::new(name, "String")),
                    "struct U256" => model.push_field(Field::new(name, "String")),
                    struct_field if struct_field.starts_with("struct ") => {
                        // field_name StructType @relation(fields: ["struct_nameId"], references: ["id"])
                        let struct_type = struct_field.strip_prefix("struct ").unwrap().to_owned();
                        let mut field = Field::new(name.clone(), struct_type);
                        let id_field_name = format!("{name}Id");
                        let mut rel = Relation::new();
                        rel.fields([id_field_name.into()].into_iter());
                        rel.references(["id".into()].into_iter());
                        field.relation(rel);
                        model.push_field(field);

                        // relation scalar field (used in the `@relation` attribute above)
                        // field_nameId Int
                        let field = Field::new(format!("{name}Id"), "Int");
                        model.push_field(field);
                    }
                    "bool" => model.push_field(Field::new(name, "Boolean")),
                    "raw untyped ptr" => model.push_field(Field::new(name, "PLACEHOLDER")),
                    "enum Option" => {
                        let option_type_application = &type_arguments.as_ref().unwrap()[0];
                        println!("INNER_TYPE: {:#?}", option_type_application);

                        let inner_decl = types.get(&option_type_application.type_id).unwrap();
                        let field_type = if inner_decl.type_field == "struct Vec" {
                            let option_type_argument = &option_type_application.type_arguments.as_ref().unwrap()[0];
                            let inner_inner_decl = types.get(&option_type_argument.type_id).unwrap();
                            if inner_inner_decl.type_field == "u8" {
                                // TODO: Prisma eqivalent of Vec<u8>
                                "String".to_string()
                            } else {
                                inner_inner_decl.type_field.clone()    
                            }
                        } else if inner_decl.type_field.starts_with("struct") {
                            inner_decl.type_field.strip_prefix("struct ").unwrap().to_owned()
                        } else if inner_decl.type_field.starts_with("enum") {
                            inner_decl.type_field.strip_prefix("struct ").unwrap().to_owned()
                        } else {
                            inner_decl.type_field.clone()
                        };
                        
                        let mut f = Field::new(name, field_type);
                        f.optional();
                        model.push_field(f);
                    },
                    "[_; 7]" => {
                        // TODO
                        continue;
                    }
                    x => unimplemented!("{x}"),
                }
            }
            println!(
                ">> MODEL:\n{}",
                (psl::reformat(&format!("{model}"), 2).unwrap())
            );
            data_model.push_model(model);
        }
    }

    let header = r#"
datasource db {
    provider = "postgresql"
    url      = "postgresql://postgres:postgres@localhost"
}

generator client {
    provider = "prisma-client-js"
}"#;

    let rendered = psl::reformat(&format!("{header}\n{data_model}"), 2).unwrap();
    println!(">> RENDERED\n{rendered}");
    rendered
}
