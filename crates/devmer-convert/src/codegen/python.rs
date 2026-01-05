//! Python code generation

use crate::error::Result;
use crate::ir::*;
use crate::ConvertOptions;
use crate::codegen::generator::GeneratedFile;
use convert_case::{Case, Casing};
use std::path::PathBuf;

/// Generate Python code from IR
pub fn generate(module: &IrModule, options: &ConvertOptions) -> Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    // Generate main file
    let main_content = generate_main(module, options);
    files.push(GeneratedFile {
        path: PathBuf::from("__main__.py"),
        content: main_content,
        is_main: true,
    });

    // Generate requirements.txt
    let requirements = generate_requirements(module);
    files.push(GeneratedFile {
        path: PathBuf::from("requirements.txt"),
        content: requirements,
        is_main: false,
    });

    // Generate pyproject.toml
    let pyproject = generate_pyproject(options);
    files.push(GeneratedFile {
        path: PathBuf::from("pyproject.toml"),
        content: pyproject,
        is_main: false,
    });

    Ok(files)
}

/// Generate main __main__.py file
fn generate_main(module: &IrModule, _options: &ConvertOptions) -> String {
    let mut output = String::new();

    // Imports
    output.push_str("import devmer\n");

    // Provider-specific imports
    let providers: std::collections::HashSet<&str> = module
        .resources
        .iter()
        .map(|r| r.provider_name())
        .collect();

    for provider in providers {
        if !provider.is_empty() {
            output.push_str(&format!("from devmer import {}\n", provider));
        }
    }

    output.push('\n');

    // Configuration/Variables
    if !module.variables.is_empty() {
        output.push_str("# Configuration\n");
        output.push_str("config = devmer.Config()\n\n");

        for var in &module.variables {
            let py_name = var.name.to_case(Case::Snake);
            let getter = if var.sensitive {
                "get_secret"
            } else {
                "get"
            };

            let default_arg = var
                .default
                .as_ref()
                .map(|d| format!(", default={}", expr_to_py(d)))
                .unwrap_or_default();

            output.push_str(&format!(
                "{} = config.{}(\"{}\"{})\n",
                py_name, getter, var.name, default_arg
            ));
        }
        output.push('\n');
    }

    // Locals
    if !module.locals.is_empty() {
        output.push_str("# Local values\n");
        for (name, value) in &module.locals {
            output.push_str(&format!(
                "{} = {}\n",
                name.to_case(Case::Snake),
                expr_to_py(value)
            ));
        }
        output.push('\n');
    }

    // Data sources
    if !module.data_sources.is_empty() {
        output.push_str("# Data sources\n");
        for data in &module.data_sources {
            output.push_str(&generate_data_source(data));
            output.push('\n');
        }
    }

    // Resources
    if !module.resources.is_empty() {
        output.push_str("# Resources\n");
        for resource in &module.resources {
            if let Some(comment) = &resource.comment {
                output.push_str(&format!("# {}\n", comment));
            }
            output.push_str(&generate_resource(resource));
            output.push('\n');
        }
    }

    // Outputs
    if !module.outputs.is_empty() {
        output.push_str("# Outputs\n");
        for output_def in &module.outputs {
            output.push_str(&format!(
                "devmer.export(\"{}\", {})\n",
                output_def.name,
                expr_to_py(&output_def.value)
            ));
        }
    }

    output
}

/// Generate a resource
fn generate_resource(resource: &IrResource) -> String {
    let mut output = String::new();

    let provider = resource.provider_name();
    let type_without_provider = resource.type_without_provider();
    let type_parts: Vec<&str> = type_without_provider.split('_').collect();
    let module_name = type_parts.first().copied().unwrap_or("resource");
    let class_name = type_parts
        .iter()
        .skip(1)
        .map(|s| s.to_case(Case::Pascal))
        .collect::<Vec<_>>()
        .join("");
    let class_name = if class_name.is_empty() {
        module_name.to_case(Case::Pascal)
    } else {
        class_name
    };

    let var_name = resource.name.to_case(Case::Snake);

    // Handle count/for_each
    if let Some(ref _count) = resource.count {
        output.push_str(&format!(
            "{} = [\n    {}.{}.{}(f\"{}_{{}}\".format(i),\n",
            var_name, provider, module_name, class_name, resource.name
        ));
    } else if let Some(ref _for_each) = resource.for_each {
        output.push_str(&format!(
            "{} = {{\n    key: {}.{}.{}(f\"{{}}-{}\".format(key),\n",
            var_name, provider, module_name, class_name, resource.name
        ));
    } else {
        output.push_str(&format!(
            "{} = {}.{}.{}(\"{}\",\n",
            var_name, provider, module_name, class_name, resource.name
        ));
    }

    // Attributes
    for (key, value) in &resource.attributes {
        let py_key = key.to_case(Case::Snake);
        output.push_str(&format!("    {}={},\n", py_key, expr_to_py(value)));
    }

    // Nested blocks
    for block in &resource.blocks {
        output.push_str(&format!(
            "    {}={},\n",
            block.block_type.to_case(Case::Snake),
            block_to_py(block)
        ));
    }

    // Resource options
    if !resource.depends_on.is_empty() {
        let deps: Vec<String> = resource
            .depends_on
            .iter()
            .map(|d| d.to_case(Case::Snake))
            .collect();
        output.push_str(&format!("    opts=devmer.ResourceOptions(depends_on=[{}]),\n", deps.join(", ")));
    }

    output.push_str(")");

    // Close count/for_each
    if resource.count.is_some() {
        output.push_str(&format!(
            "\n    for i in range({})\n]",
            resource.count.as_ref().map(expr_to_py).unwrap_or_default()
        ));
    } else if resource.for_each.is_some() {
        output.push_str(&format!(
            "\n    for key, value in {}.items()\n}}",
            resource.for_each.as_ref().map(expr_to_py).unwrap_or_default()
        ));
    }

    output.push('\n');
    output
}

/// Generate a data source
fn generate_data_source(data: &IrDataSource) -> String {
    let provider = data.data_type.split('_').next().unwrap_or("data");
    let type_parts: Vec<&str> = data.data_type.splitn(2, '_').nth(1).unwrap_or("").split('_').collect();
    let module_name = type_parts.first().copied().unwrap_or("data");
    let func_name = format!(
        "get_{}",
        type_parts.iter().skip(1).map(|s| s.to_case(Case::Snake)).collect::<Vec<_>>().join("_")
    );

    let var_name = data.name.to_case(Case::Snake);

    let mut output = format!("{} = {}.{}.{}(\n", var_name, provider, module_name, func_name);

    for (key, value) in &data.attributes {
        let py_key = key.to_case(Case::Snake);
        output.push_str(&format!("    {}={},\n", py_key, expr_to_py(value)));
    }

    output.push_str(")\n");
    output
}

/// Convert a block to Python
fn block_to_py(block: &IrBlock) -> String {
    let mut output = String::from("{\n");

    for (key, value) in &block.attributes {
        let py_key = key.to_case(Case::Snake);
        output.push_str(&format!("        \"{}\": {},\n", py_key, expr_to_py(value)));
    }

    output.push_str("    }");
    output
}

/// Convert an expression to Python
fn expr_to_py(expr: &IrExpression) -> String {
    match expr {
        IrExpression::Null => "None".to_string(),
        IrExpression::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        IrExpression::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        IrExpression::String(s) => format!("\"{}\"", escape_string(s)),
        IrExpression::List(items) => {
            let items_str: Vec<String> = items.iter().map(expr_to_py).collect();
            format!("[{}]", items_str.join(", "))
        }
        IrExpression::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k.to_case(Case::Snake), expr_to_py(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
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
        IrExpression::DataRef { name, attribute, .. } => {
            let var_name = name.to_case(Case::Snake);
            match attribute {
                Some(attr) => format!("{}.{}", var_name, attr.to_case(Case::Snake)),
                None => var_name,
            }
        }
        IrExpression::ModuleRef { name, output } => {
            format!("{}.{}", name.to_case(Case::Snake), output.to_case(Case::Snake))
        }
        IrExpression::EachRef(key) => format!("{}", key),
        IrExpression::CountIndex => "i".to_string(),
        IrExpression::TerraformWorkspace => "devmer.get_stack()".to_string(),
        IrExpression::FunctionCall { name, args } => {
            let py_func = map_function_to_py(name);
            let args_str: Vec<String> = args.iter().map(expr_to_py).collect();
            format!("{}({})", py_func, args_str.join(", "))
        }
        IrExpression::Conditional { condition, true_result, false_result } => {
            format!(
                "{} if {} else {}",
                expr_to_py(true_result),
                expr_to_py(condition),
                expr_to_py(false_result)
            )
        }
        IrExpression::ForExpr { value_var, collection, value_expr, condition, is_object, .. } => {
            let coll = expr_to_py(collection);
            let val = expr_to_py(value_expr);
            let cond = condition
                .as_ref()
                .map(|c| format!(" if {}", expr_to_py(c)))
                .unwrap_or_default();

            if *is_object {
                format!("{{/* key */: {} for {} in {}{}}}", val, value_var, coll, cond)
            } else {
                format!("[{} for {} in {}{}]", val, value_var, coll, cond)
            }
        }
        IrExpression::BinaryOp { left, op, right } => {
            format!(
                "({} {} {})",
                expr_to_py(left),
                op.to_string(crate::codegen::Language::Python),
                expr_to_py(right)
            )
        }
        IrExpression::UnaryOp { op, expr } => {
            let op_str = match op {
                UnaryOperator::Not => "not ",
                UnaryOperator::Neg => "-",
            };
            format!("{}{}", op_str, expr_to_py(expr))
        }
        IrExpression::Template(parts) => {
            let mut result = String::from("f\"");
            for part in parts {
                match part {
                    TemplatePart::Literal(s) => result.push_str(&escape_string(s)),
                    TemplatePart::Interpolation(expr) => {
                        result.push_str(&format!("{{{}}}", expr_to_py(expr)));
                    }
                    TemplatePart::Directive(d) => {
                        result.push_str(&format!("/* directive: {} */", d));
                    }
                }
            }
            result.push('"');
            result
        }
        IrExpression::Heredoc { content, .. } => {
            format!("\"\"\"{}\"\"\"", content)
        }
        _ => format!("# TODO: {:#?}", expr),
    }
}

/// Map Terraform function to Python equivalent
fn map_function_to_py(name: &str) -> String {
    match name {
        "concat" => "list".to_string(),
        "join" => "\"\".join".to_string(),
        "split" => "str.split".to_string(),
        "length" => "len".to_string(),
        "lower" => "str.lower".to_string(),
        "upper" => "str.upper".to_string(),
        "trim" => "str.strip".to_string(),
        "jsonencode" => "json.dumps".to_string(),
        "jsondecode" => "json.loads".to_string(),
        "tolist" => "list".to_string(),
        "toset" => "set".to_string(),
        "tomap" => "dict".to_string(),
        "merge" => "devmer.merge".to_string(),
        "keys" => "dict.keys".to_string(),
        "values" => "dict.values".to_string(),
        "flatten" => "devmer.flatten".to_string(),
        "sort" => "sorted".to_string(),
        "reverse" => "list(reversed".to_string(),
        "contains" => "devmer.contains".to_string(),
        "range" => "range".to_string(),
        _ => format!("devmer.{}", name.to_case(Case::Snake)),
    }
}

/// Escape a string for Python
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Generate requirements.txt
fn generate_requirements(module: &IrModule) -> String {
    let mut deps = vec!["devmer>=0.1.0".to_string()];

    let providers: std::collections::HashSet<&str> = module
        .resources
        .iter()
        .map(|r| r.provider_name())
        .collect();

    for provider in providers {
        if !provider.is_empty() {
            deps.push(format!("devmer-{}>=0.1.0", provider));
        }
    }

    deps.join("\n")
}

/// Generate pyproject.toml
fn generate_pyproject(options: &ConvertOptions) -> String {
    let project_name = options
        .project_name
        .as_deref()
        .unwrap_or("devmer-project");

    format!(
        r#"[project]
name = "{}"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "devmer>=0.1.0",
]

[tool.ruff]
line-length = 100
"#,
        project_name
    )
}
