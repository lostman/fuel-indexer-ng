use fuel_abi_types::abi::program::TypeDeclaration;
use fuels::types::{enum_variants::EnumVariants, Token};

pub trait TypeDeclarationExt {
    fn struct_or_enum_name(&self) -> Option<String>;
    fn decl_fields(&self, abi: &crate::ABI) -> Vec<TypeDeclaration>;
    fn is_u256(&self) -> bool;
    fn is_enum(&self) -> bool;
    fn is_struct(&self) -> bool;
    fn is_array(&self) -> bool;
    fn has_nested_struct(&self, abi: &crate::ABI) -> bool;
    fn has_nested_enum(&self, abi: &crate::ABI) -> bool;
    fn has_nested_array(&self, abi: &crate::ABI) -> bool;
}

impl TypeDeclarationExt for TypeDeclaration {
    fn struct_or_enum_name(&self) -> Option<String> {
        self.type_field
            .strip_prefix("struct ")
            .or(self.type_field.strip_prefix("enum "))
            .map(std::string::ToString::to_string)
    }

    fn decl_fields(&self, abi: &crate::ABI) -> Vec<TypeDeclaration> {
        let mut result = vec![];
        for field in self.components.as_ref().unwrap() {
            let field_decl = abi.type_declaration(field.type_id);
            result.push(field_decl)
        }
        result
    }

    fn is_u256(&self) -> bool {
        self.type_field.starts_with("struct U256")
    }

    fn is_struct(&self) -> bool {
        self.type_field.starts_with("struct")
    }

    fn is_enum(&self) -> bool {
        self.type_field.starts_with("enum")
    }

    fn is_array(&self) -> bool {
        self.components
            .as_ref()
            .map(|cs| cs[0].name == "__array_element".to_string())
            .unwrap_or(false)
    }

    fn has_nested_struct(&self, abi: &crate::ABI) -> bool {
        self.decl_fields(abi)
            .iter()
            .any(TypeDeclarationExt::is_struct)
    }

    fn has_nested_enum(&self, abi: &crate::ABI) -> bool {
        self.decl_fields(abi)
            .iter()
            .any(TypeDeclarationExt::is_enum)
    }

    fn has_nested_array(&self, abi: &crate::ABI) -> bool {
        self.decl_fields(abi)
            .iter()
            .any(TypeDeclarationExt::is_array)
    }
}

pub trait TokenExt {
    fn as_struct(&self) -> &Vec<Token>;
    fn as_enum(&self) -> (u64, Token, EnumVariants);
    fn as_array(&self) -> &Vec<Token>;
    fn is_struct(&self) -> bool;
    fn is_enum(&self) -> bool;
    fn is_array(&self) -> bool;
}

impl TokenExt for Token {
    fn as_struct(&self) -> &Vec<Token> {
        match self {
            Token::Struct(xs) => xs,
            _ => panic!("Expected Token::Struct but got {self:#?}"),
        }
    }

    fn as_enum(&self) -> (u64, Token, EnumVariants) {
        match self {
            Token::Enum(x) => *x.clone(),
            _ => panic!("Expected Token::Enum but got {self:#?}"),
        }
    }

    fn as_array(&self) -> &Vec<Token> {
        match self {
            Token::Array(xs) => xs,
            _ => panic!("Expected Token::Struct but got {self:#?}"),
        }
    }

    fn is_struct(&self) -> bool {
        match self {
            Token::Struct(_) => true,
            _ => false,
        }
    }

    fn is_enum(&self) -> bool {
        match self {
            Token::Enum(_) => true,
            _ => false,
        }
    }

    fn is_array(&self) -> bool {
        match self {
            Token::Array(_) => true,
            _ => false,
        }
    }
}
