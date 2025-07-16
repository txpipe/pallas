use cddl::ast::CDDL;
use cddl::parser::cddl_from_str;
use convert_case::Case;
use convert_case::Casing as _;
use proc_macro2::TokenStream;
use quote::format_ident;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun this script if the CDDL file changes
    println!("cargo:rerun-if-changed=schema.cddl");

    // Read the CDDL file
    let cddl_content = fs::read_to_string("cddls/example.cddl")?;

    // Parse the CDDL content into an AST
    let ast: CDDL = cddl_from_str(&cddl_content, true)?;

    // Get the output directory from cargo
    let out_dir = env::var("OUT_DIR")?;
    //let dest_path = Path::new(&out_dir).join("generated.rs");
    let dest_path = Path::new("generated.rs");

    // Generate your Rust code here based on the AST
    let generated_code = generate_code_from_ast(&ast);

    // Write the generated code to a file
    fs::write(dest_path, generated_code)?;

    Ok(())
}

fn generate_field(ast: &cddl::ast::GroupEntry) -> TokenStream {
    match ast {
        cddl::ast::GroupEntry::ValueMemberKey { ge, .. } => {
            let field_name = ge
                .member_key
                .as_ref()
                .and_then(|key| {
                    if let cddl::ast::MemberKey::Bareword { ident, .. } = key {
                        Some(format_ident!("{}", ident.ident.to_case(Case::Snake)))
                    } else {
                        None
                    }
                })
                .unwrap();

            // Get field type (simplified - you'll want to expand this)
            let field_type = match &ge.entry_type.type_choices.get(0).unwrap().type1.type2 {
                cddl::ast::Type2::Typename { ident, .. } if ident.ident == "uint" => {
                    quote::quote!(u64)
                }
                // Add more type mappings as needed
                _ => quote::quote!(()), // Default case
            };

            quote::quote! {
                pub #field_name: #field_type
            }
        }
        _ => todo!(),
    }
}

fn generate_struct_from_rule(rule: &cddl::ast::TypeRule) -> TokenStream {
    // Get the rule name from the AST
    let rule_name = rule.name.ident.to_case(Case::Pascal);
    let struct_name = format_ident!("{}", rule_name);

    // Extract fields from the group entries
    let type2 = &rule.value.type_choices.get(0).unwrap().type1.type2;

    let fields: Vec<_> = match type2 {
        cddl::ast::Type2::Array { group, .. } => group.group_choices[0]
            .group_entries
            .iter()
            .map(|(entry, _comma)| generate_field(entry)),
        _ => todo!(),
    }
    .collect();

    quote::quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub struct #struct_name {
            #(#fields,)*
        }
    }
}

fn generate_code_from_ast(ast: &CDDL) -> String {
    let mut output = TokenStream::new();

    for rule in &ast.rules {
        let generated = match rule {
            cddl::ast::Rule::Type { rule, .. } => generate_struct_from_rule(rule),
            cddl::ast::Rule::Group { rule, .. } => todo!(),
        };

        output.extend(generated);
    }

    let syntax_tree = syn::parse_file(&output.to_string()).unwrap();
    prettyplease::unparse(&syntax_tree)
}
