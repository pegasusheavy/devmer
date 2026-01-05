//! Go code generation

use crate::error::Result;
use crate::ir::*;
use crate::ConvertOptions;
use crate::codegen::generator::GeneratedFile;
use convert_case::{Case, Casing};
use std::path::PathBuf;

/// Generate Go code from IR
pub fn generate(module: &IrModule, options: &ConvertOptions) -> Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    // Generate main file
    let main_content = generate_main(module, options);
    files.push(GeneratedFile {
        path: PathBuf::from("main.go"),
        content: main_content,
        is_main: true,
    });

    // Generate go.mod
    let go_mod = generate_go_mod(options);
    files.push(GeneratedFile {
        path: PathBuf::from("go.mod"),
        content: go_mod,
        is_main: false,
    });

    Ok(files)
}

/// Generate main.go file
fn generate_main(module: &IrModule, _options: &ConvertOptions) -> String {
    let mut output = String::new();

    output.push_str("package main\n\n");

    // Imports
    output.push_str("import (\n");
    output.push_str("\t\"github.com/devmer/sdk-go/devmer\"\n");

    // Provider-specific imports
    let providers: std::collections::HashSet<&str> = module
        .resources
        .iter()
        .map(|r| r.provider_name())
        .collect();

    for provider in providers {
        if !provider.is_empty() {
            output.push_str(&format!(
                "\t\"github.com/devmer/sdk-go/devmer/{}\"\n",
                provider
            ));
        }
    }

    output.push_str(")\n\n");

    // Main function
    output.push_str("func main() {\n");
    output.push_str("\tdevmer.Run(func(ctx *devmer.Context) error {\n");

    // Configuration/Variables
    if !module.variables.is_empty() {
        output.push_str("\t\t// Configuration\n");
        output.push_str("\t\tconfig := devmer.NewConfig(ctx)\n\n");

        for var in &module.variables {
            let go_name = var.name.to_case(Case::Camel);
            let go_type = var.var_type.as_ref().map(|t| t.to_go()).unwrap_or("string".to_string());

            output.push_str(&format!(
                "\t\t{} := config.Get{}(\"{}\")\n",
                go_name,
                go_type.to_case(Case::Pascal),
                var.name
            ));
        }
        output.push('\n');
    }

    // Locals
    if !module.locals.is_empty() {
        output.push_str("\t\t// Local values\n");
        for (name, value) in &module.locals {
            output.push_str(&format!(
                "\t\t{} := {}\n",
                name.to_case(Case::Camel),
                expr_to_go(value)
            ));
        }
        output.push('\n');
    }

    // Resources
    if !module.resources.is_empty() {
        output.push_str("\t\t// Resources\n");
        for resource in &module.resources {
            if let Some(comment) = &resource.comment {
                output.push_str(&format!("\t\t// {}\n", comment));
            }
            output.push_str(&generate_resource(resource));
            output.push('\n');
        }
    }

    // Outputs
    if !module.outputs.is_empty() {
        output.push_str("\t\t// Outputs\n");
        for output_def in &module.outputs {
            output.push_str(&format!(
                "\t\tctx.Export(\"{}\", {})\n",
                output_def.name,
                expr_to_go(&output_def.value)
            ));
        }
    }

    output.push_str("\n\t\treturn nil\n");
    output.push_str("\t})\n");
    output.push_str("}\n");

    output
}

/// Generate a resource
fn generate_resource(resource: &IrResource) -> String {
    let mut output = String::new();

    let provider = resource.provider_name();
    let type_without_provider = resource.type_without_provider();
    let type_parts: Vec<&str> = type_without_provider.split('_').collect();
    let module_name = type_parts.first().copied().unwrap_or("resource");
    let struct_name = type_parts
        .iter()
        .skip(1)
        .map(|s| s.to_case(Case::Pascal))
        .collect::<Vec<_>>()
        .join("");
    let struct_name = if struct_name.is_empty() {
        module_name.to_case(Case::Pascal)
    } else {
        struct_name
    };

    let var_name = resource.name.to_case(Case::Camel);

    output.push_str(&format!(
        "\t\t{}, err := {}.New{}(ctx, \"{}\", &{}.{}Args{{\n",
        var_name, provider, struct_name, resource.name, provider, struct_name
    ));

    // Attributes
    for (key, value) in &resource.attributes {
        let go_key = key.to_case(Case::Pascal);
        output.push_str(&format!("\t\t\t{}: {},\n", go_key, expr_to_go_ptr(value)));
    }

    output.push_str("\t\t}");

    // Resource options
    if !resource.depends_on.is_empty() {
        let deps: Vec<String> = resource
            .depends_on
            .iter()
            .map(|d| d.to_case(Case::Camel))
            .collect();
        output.push_str(&format!(
            ", devmer.DependsOn([]devmer.Resource{{{}}})",
            deps.join(", ")
        ));
    }

    output.push_str(")\n");
    output.push_str("\t\tif err != nil {\n\t\t\treturn err\n\t\t}\n");

    // Suppress unused variable warning
    output.push_str(&format!("\t\t_ = {}\n", var_name));

    output
}

/// Convert an expression to Go
fn expr_to_go(expr: &IrExpression) -> String {
    match expr {
        IrExpression::Null => "nil".to_string(),
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
            let items_str: Vec<String> = items.iter().map(expr_to_go).collect();
            format!("[]interface{{}}{{{}}}", items_str.join(", "))
        }
        IrExpression::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, expr_to_go(v)))
                .collect();
            format!("map[string]interface{{}}{{{}}}", items.join(", "))
        }
        IrExpression::VarRef(name) => name.to_case(Case::Camel),
        IrExpression::LocalRef(name) => name.to_case(Case::Camel),
        IrExpression::ResourceRef { name, attribute, .. } => {
            let var_name = name.to_case(Case::Camel);
            match attribute {
                Some(attr) => format!("{}.{}", var_name, attr.to_case(Case::Pascal)),
                None => var_name,
            }
        }
        IrExpression::TerraformWorkspace => "ctx.Stack()".to_string(),
        IrExpression::FunctionCall { name, args } => {
            let go_func = map_function_to_go(name);
            let args_str: Vec<String> = args.iter().map(expr_to_go).collect();
            format!("{}({})", go_func, args_str.join(", "))
        }
        IrExpression::Conditional { condition, true_result, false_result } => {
            // Go doesn't have ternary, need helper
            format!(
                "devmer.If({}, {}, {})",
                expr_to_go(condition),
                expr_to_go(true_result),
                expr_to_go(false_result)
            )
        }
        IrExpression::BinaryOp { left, op, right } => {
            format!(
                "({} {} {})",
                expr_to_go(left),
                op.to_string(crate::codegen::Language::Go),
                expr_to_go(right)
            )
        }
        IrExpression::Template(parts) => {
            let mut format_str = String::new();
            let mut args = Vec::new();

            for part in parts {
                match part {
                    TemplatePart::Literal(s) => format_str.push_str(&escape_string(s)),
                    TemplatePart::Interpolation(expr) => {
                        format_str.push_str("%v");
                        args.push(expr_to_go(expr));
                    }
                    _ => {}
                }
            }

            if args.is_empty() {
                format!("\"{}\"", format_str)
            } else {
                format!("fmt.Sprintf(\"{}\", {})", format_str, args.join(", "))
            }
        }
        _ => format!("/* TODO: {:?} */", expr),
    }
}

/// Convert an expression to Go pointer type
fn expr_to_go_ptr(expr: &IrExpression) -> String {
    match expr {
        IrExpression::String(s) => format!("devmer.String(\"{}\")", escape_string(s)),
        IrExpression::Bool(b) => format!("devmer.Bool({})", b),
        IrExpression::Number(n) => {
            if n.fract() == 0.0 {
                format!("devmer.Int({})", *n as i64)
            } else {
                format!("devmer.Float64({})", n)
            }
        }
        IrExpression::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, expr_to_go_ptr(v)))
                .collect();
            format!("devmer.Map{{{}}}", items.join(", "))
        }
        IrExpression::List(items) => {
            let items_str: Vec<String> = items.iter().map(expr_to_go_ptr).collect();
            format!("devmer.Array{{{}}}", items_str.join(", "))
        }
        _ => expr_to_go(expr),
    }
}

/// Map Terraform function to Go equivalent
fn map_function_to_go(name: &str) -> String {
    match name {
        "concat" => "append".to_string(),
        "join" => "strings.Join".to_string(),
        "split" => "strings.Split".to_string(),
        "length" => "len".to_string(),
        "lower" => "strings.ToLower".to_string(),
        "upper" => "strings.ToUpper".to_string(),
        "trim" => "strings.TrimSpace".to_string(),
        "jsonencode" => "json.Marshal".to_string(),
        "jsondecode" => "json.Unmarshal".to_string(),
        "format" => "fmt.Sprintf".to_string(),
        _ => format!("devmer.{}", name.to_case(Case::Pascal)),
    }
}

/// Escape a string for Go
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Generate go.mod
fn generate_go_mod(options: &ConvertOptions) -> String {
    let project_name = options
        .project_name
        .as_deref()
        .unwrap_or("devmer-project");

    format!(
        r#"module {}

go 1.21

require github.com/devmer/sdk-go v0.1.0
"#,
        project_name
    )
}
