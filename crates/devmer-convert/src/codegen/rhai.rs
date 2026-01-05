//! Rhai script code generation

use crate::error::Result;
use crate::ir::*;
use crate::ConvertOptions;
use crate::codegen::generator::GeneratedFile;
use convert_case::{Case, Casing};
use std::path::PathBuf;

/// Generate Rhai code from IR
pub fn generate(module: &IrModule, _options: &ConvertOptions) -> Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    // Generate main file
    let main_content = generate_main(module);
    files.push(GeneratedFile {
        path: PathBuf::from("main.rhai"),
        content: main_content,
        is_main: true,
    });

    Ok(files)
}

/// Generate main.rhai file
fn generate_main(module: &IrModule) -> String {
    let mut output = String::new();

    output.push_str("// Devmer Rhai Script\n");
    output.push_str("// Converted from Terraform/OpenTofu\n\n");

    // Configuration/Variables
    if !module.variables.is_empty() {
        output.push_str("// Configuration\n");
        for var in &module.variables {
            let rhai_name = var.name.to_case(Case::Snake);
            let default_val = var
                .default
                .as_ref()
                .map(expr_to_rhai)
                .unwrap_or_else(|| "()".to_string());

            output.push_str(&format!(
                "let {} = config(\"{}\", {});\n",
                rhai_name, var.name, default_val
            ));
        }
        output.push('\n');
    }

    // Locals
    if !module.locals.is_empty() {
        output.push_str("// Local values\n");
        for (name, value) in &module.locals {
            output.push_str(&format!(
                "let {} = {};\n",
                name.to_case(Case::Snake),
                expr_to_rhai(value)
            ));
        }
        output.push('\n');
    }

    // Resources
    if !module.resources.is_empty() {
        output.push_str("// Resources\n");
        for resource in &module.resources {
            if let Some(comment) = &resource.comment {
                output.push_str(&format!("// {}\n", comment));
            }
            output.push_str(&generate_resource(resource));
            output.push('\n');
        }
    }

    // Outputs
    if !module.outputs.is_empty() {
        output.push_str("// Outputs\n");
        for output_def in &module.outputs {
            output.push_str(&format!(
                "export(\"{}\", {});\n",
                output_def.name,
                expr_to_rhai(&output_def.value)
            ));
        }
    }

    output
}

/// Generate a resource
fn generate_resource(resource: &IrResource) -> String {
    let mut output = String::new();

    let provider = resource.provider_name();
    let resource_type = resource.type_without_provider().replace('_', "::");

    let var_name = resource.name.to_case(Case::Snake);

    output.push_str(&format!(
        "let {} = {}::{}(\"{}\", #{{\n",
        var_name, provider, resource_type, resource.name
    ));

    // Attributes
    for (key, value) in &resource.attributes {
        let rhai_key = key.to_case(Case::Snake);
        output.push_str(&format!("    {}: {},\n", rhai_key, expr_to_rhai(value)));
    }

    // Nested blocks
    for block in &resource.blocks {
        output.push_str(&format!(
            "    {}: {},\n",
            block.block_type.to_case(Case::Snake),
            block_to_rhai(block)
        ));
    }

    output.push_str("});\n");

    output
}

/// Convert a block to Rhai
fn block_to_rhai(block: &IrBlock) -> String {
    let mut output = String::from("#{\n");

    for (key, value) in &block.attributes {
        let rhai_key = key.to_case(Case::Snake);
        output.push_str(&format!("        {}: {},\n", rhai_key, expr_to_rhai(value)));
    }

    output.push_str("    }");
    output
}

/// Convert an expression to Rhai
fn expr_to_rhai(expr: &IrExpression) -> String {
    match expr {
        IrExpression::Null => "()".to_string(),
        IrExpression::Bool(b) => b.to_string(),
        IrExpression::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        IrExpression::String(s) => format!("\"{}\"", escape_string(s)),
        IrExpression::List(items) => {
            let items_str: Vec<String> = items.iter().map(expr_to_rhai).collect();
            format!("[{}]", items_str.join(", "))
        }
        IrExpression::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k.to_case(Case::Snake), expr_to_rhai(v)))
                .collect();
            format!("#{{{}}}", items.join(", "))
        }
        IrExpression::VarRef(name) => name.to_case(Case::Snake),
        IrExpression::LocalRef(name) => name.to_case(Case::Snake),
        IrExpression::ResourceRef { name, attribute, .. } => {
            let var_name = name.to_case(Case::Snake);
            match attribute {
                Some(attr) => format!("{}.{}", var_name, attr.to_case(Case::Snake)),
                None => var_name,
            }
        }
        IrExpression::TerraformWorkspace => "stack()".to_string(),
        IrExpression::FunctionCall { name, args } => {
            let rhai_func = name.to_case(Case::Snake);
            let args_str: Vec<String> = args.iter().map(expr_to_rhai).collect();
            format!("{}({})", rhai_func, args_str.join(", "))
        }
        IrExpression::Conditional { condition, true_result, false_result } => {
            format!(
                "if {} {{ {} }} else {{ {} }}",
                expr_to_rhai(condition),
                expr_to_rhai(true_result),
                expr_to_rhai(false_result)
            )
        }
        IrExpression::ForExpr { value_var, collection, value_expr, condition, .. } => {
            let coll = expr_to_rhai(collection);
            let val = expr_to_rhai(value_expr);
            let cond = condition
                .as_ref()
                .map(|c| format!(".filter(|{}| {})", value_var, expr_to_rhai(c)))
                .unwrap_or_default();

            format!("{}{}.map(|{}| {})", coll, cond, value_var, val)
        }
        IrExpression::BinaryOp { left, op, right } => {
            format!(
                "({} {} {})",
                expr_to_rhai(left),
                op.to_string(crate::codegen::Language::Rhai),
                expr_to_rhai(right)
            )
        }
        IrExpression::UnaryOp { op, expr } => {
            let op_str = match op {
                UnaryOperator::Not => "!",
                UnaryOperator::Neg => "-",
            };
            format!("{}{}", op_str, expr_to_rhai(expr))
        }
        IrExpression::Template(parts) => {
            // Rhai uses string interpolation with $
            let mut result = String::from("`");
            for part in parts {
                match part {
                    TemplatePart::Literal(s) => result.push_str(&escape_template_string(s)),
                    TemplatePart::Interpolation(expr) => {
                        result.push_str(&format!("${{{}}}", expr_to_rhai(expr)));
                    }
                    TemplatePart::Directive(d) => {
                        result.push_str(&format!("/* {} */", d));
                    }
                }
            }
            result.push('`');
            result
        }
        IrExpression::Heredoc { content, .. } => {
            format!("`{}`", escape_template_string(content))
        }
        _ => format!("/* TODO: {:?} */", expr),
    }
}

/// Escape a string for Rhai
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a template string for Rhai
fn escape_template_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}
