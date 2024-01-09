use datamodel_renderer::datamodel::{Field, IdFieldDefinition, Model, Relation};
use datamodel_renderer::{self, Datamodel};

use fuel_abi_types::abi::program::{TypeApplication, TypeDeclaration};

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
            println!(">> PROCESSING STRUCT: {name}");
            let mut model = Model::new(name.to_string());
            let mut id_field = Field::new("id", "Int");
            id_field.id(IdFieldDefinition::new());
            model.push_field(id_field);
            for TypeApplication { name, type_id, .. } in components.as_ref().unwrap() {
                let field_decl = types.get(&type_id).unwrap();
                match field_decl.type_field.as_str() {
                    "u32" => model.push_field(Field::new(name, "Int")),
                    "u64" => model.push_field(Field::new(name, "Int")),
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
                    _ => unimplemented!(),
                }
            }
            println!(
                ">> MODEL:\n{}",
                (psl::reformat(&format!("{model}"), 2).unwrap())
            );
            data_model.push_model(model);
        }
    }

    let rendered = psl::reformat(&format!("{data_model}"), 2).unwrap();
    println!(">> RENDERED\n{rendered}");
    rendered
}
