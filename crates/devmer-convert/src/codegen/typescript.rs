//! TypeScript code generation

use crate::error::Result;
use crate::ir::*;
use crate::ConvertOptions;
use crate::codegen::generator::GeneratedFile;
use convert_case::{Case, Casing};
use std::path::PathBuf;

/// Generate TypeScript code from IR
pub fn generate(module: &IrModule, options: &ConvertOptions) -> Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    // Generate main file
    let main_content = generate_main(module, options);
    files.push(GeneratedFile {
        path: PathBuf::from("index.ts"),
        content: main_content,
        is_main: true,
    });

    // Generate types file if there are complex types
    if !module.variables.is_empty() {
        let types_content = generate_types(module);
        files.push(GeneratedFile {
            path: PathBuf::from("types.ts"),
            content: types_content,
            is_main: false,
        });
    }

    // Generate package.json
    let package_json = generate_package_json(module, options);
    files.push(GeneratedFile {
        path: PathBuf::from("package.json"),
        content: package_json,
        is_main: false,
    });

    // Generate tsconfig.json
    files.push(GeneratedFile {
        path: PathBuf::from("tsconfig.json"),
        content: generate_tsconfig(),
        is_main: false,
    });

    Ok(files)
}

/// Generate main index.ts file
fn generate_main(module: &IrModule, options: &ConvertOptions) -> String {
    let mut output = String::new();

    // Imports
    output.push_str("import * as devmer from \"@devmer/sdk\";\n");

    // Provider-specific imports
    let providers: std::collections::HashSet<&str> = module
        .resources
        .iter()
        .map(|r| r.provider_name())
        .chain(module.data_sources.iter().map(|d| d.data_type.split('_').next().unwrap_or("")))
        .collect();

    for provider in providers {
        if !provider.is_empty() {
            output.push_str(&format!(
                "import * as {} from \"@devmer/{}\";\n",
                provider, provider
            ));
        }
    }

    output.push('\n');

    // Configuration/Variables
    if !module.variables.is_empty() {
        output.push_str("// Configuration\n");
        output.push_str("const config = new devmer.Config();\n\n");

        for var in &module.variables {
            let ts_type = var.var_type.as_ref().map(|t| t.to_typescript()).unwrap_or("string".to_string());
            let getter = if var.sensitive {
                "getSecret"
            } else {
                "get"
            };

            output.push_str(&format!(
                "const {} = config.{}<{}>(\"{}\"{});\n",
                var.name.to_case(Case::Camel),
                getter,
                ts_type,
                var.name,
                var.default.as_ref().map(|d| format!(", {{ default: {} }}", expr_to_ts(d))).unwrap_or_default()
            ));
        }
        output.push('\n');
    }

    // Locals
    if !module.locals.is_empty() {
        output.push_str("// Local values\n");
        for (name, value) in &module.locals {
            output.push_str(&format!(
                "const {} = {};\n",
                name.to_case(Case::Camel),
                expr_to_ts(value)
            ));
        }
        output.push('\n');
    }

    // Data sources
    if !module.data_sources.is_empty() {
        output.push_str("// Data sources\n");
        for data in &module.data_sources {
            output.push_str(&generate_data_source(data));
            output.push('\n');
        }
    }

    // Resources
    if !module.resources.is_empty() {
        output.push_str("// Resources\n");
        for resource in &module.resources {
            if let Some(comment) = &resource.comment {
                output.push_str(&format!("// {}\n", comment));
            }
            output.push_str(&generate_resource(resource, options));
            output.push('\n');
        }
    }

    // Module calls
    for mod_call in &module.modules {
        output.push_str(&generate_module_call(mod_call));
        output.push('\n');
    }

    // Outputs
    if !module.outputs.is_empty() {
        output.push_str("// Outputs\n");
        for output_def in &module.outputs {
            let export_name = output_def.name.to_case(Case::Camel);
            output.push_str(&format!(
                "export const {} = {};\n",
                export_name,
                expr_to_ts(&output_def.value)
            ));
        }
    }

    output
}

/// Generate a resource
fn generate_resource(resource: &IrResource, _options: &ConvertOptions) -> String {
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

    let var_name = resource.name.to_case(Case::Camel);

    // Handle count/for_each
    let (prefix, suffix) = if resource.count.is_some() {
        (
            format!(
                "const {} = [];\nfor (let i = 0; i < {}; i++) {{\n  {}.push(",
                var_name,
                resource
                    .count
                    .as_ref()
                    .map(expr_to_ts)
                    .unwrap_or_default(),
                var_name
            ),
            ");\n}".to_string(),
        )
    } else if resource.for_each.is_some() {
        (
            format!(
                "const {} = new Map();\nfor (const [key, value] of Object.entries({})) {{\n  {}.set(key, ",
                var_name,
                resource
                    .for_each
                    .as_ref()
                    .map(expr_to_ts)
                    .unwrap_or_default(),
                var_name
            ),
            ");\n}".to_string(),
        )
    } else {
        (format!("const {} = ", var_name), ";".to_string())
    };

    output.push_str(&prefix);
    output.push_str(&format!(
        "new {}.{}.{}(\"{}\", {{\n",
        provider, module_name, class_name, resource.name
    ));

    // Attributes
    for (key, value) in &resource.attributes {
        let ts_key = key.to_case(Case::Camel);
        output.push_str(&format!("    {}: {},\n", ts_key, expr_to_ts(value)));
    }

    // Nested blocks
    for block in &resource.blocks {
        output.push_str(&format!(
            "    {}: {},\n",
            block.block_type.to_case(Case::Camel),
            block_to_ts(block)
        ));
    }

    output.push_str("}");

    // Resource options
    let mut opts = vec![];
    if !resource.depends_on.is_empty() {
        let deps: Vec<String> = resource
            .depends_on
            .iter()
            .map(|d| d.to_case(Case::Camel))
            .collect();
        opts.push(format!("dependsOn: [{}]", deps.join(", ")));
    }
    if let Some(ref lifecycle) = resource.lifecycle {
        if lifecycle.prevent_destroy {
            opts.push("protect: true".to_string());
        }
        if !lifecycle.ignore_changes.is_empty() {
            opts.push(format!(
                "ignoreChanges: [{}]",
                lifecycle
                    .ignore_changes
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    if !opts.is_empty() {
        output.push_str(&format!(", {{ {} }}", opts.join(", ")));
    }

    output.push(')');
    output.push_str(&suffix);

    output
}

/// Generate a data source
fn generate_data_source(data: &IrDataSource) -> String {
    let provider = data.data_type.split('_').next().unwrap_or("data");
    let type_parts: Vec<&str> = data.data_type.splitn(2, '_').nth(1).unwrap_or("").split('_').collect();
    let module_name = type_parts.first().copied().unwrap_or("data");
    let class_name = type_parts
        .iter()
        .skip(1)
        .map(|s| s.to_case(Case::Pascal))
        .collect::<Vec<_>>()
        .join("");
    let class_name = if class_name.is_empty() {
        module_name.to_case(Case::Pascal)
    } else {
        format!("get{}", class_name)
    };

    let var_name = data.name.to_case(Case::Camel);

    let mut output = format!(
        "const {} = {}.{}.{}({{\n",
        var_name, provider, module_name, class_name
    );

    for (key, value) in &data.attributes {
        let ts_key = key.to_case(Case::Camel);
        output.push_str(&format!("    {}: {},\n", ts_key, expr_to_ts(value)));
    }

    output.push_str("});\n");

    output
}

/// Generate a module call
fn generate_module_call(mod_call: &IrModuleCall) -> String {
    let var_name = mod_call.name.to_case(Case::Camel);

    let mut output = format!(
        "// Module: {}\n// Source: {}\n",
        mod_call.name, mod_call.source
    );
    output.push_str(&format!(
        "// TODO: Convert module call to component or inline resources\nconst {} = undefined; // Module call not yet supported\n",
        var_name
    ));

    output
}

/// Convert a block to TypeScript
fn block_to_ts(block: &IrBlock) -> String {
    let mut output = String::from("{\n");

    for (key, value) in &block.attributes {
        let ts_key = key.to_case(Case::Camel);
        output.push_str(&format!("        {}: {},\n", ts_key, expr_to_ts(value)));
    }

    for nested in &block.blocks {
        output.push_str(&format!(
            "        {}: {},\n",
            nested.block_type.to_case(Case::Camel),
            block_to_ts(nested)
        ));
    }

    output.push_str("    }");
    output
}

/// Convert an expression to TypeScript
fn expr_to_ts(expr: &IrExpression) -> String {
    match expr {
        IrExpression::Null => "null".to_string(),
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
            let items_str: Vec<String> = items.iter().map(expr_to_ts).collect();
            format!("[{}]", items_str.join(", "))
        }
        IrExpression::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k.to_case(Case::Camel), expr_to_ts(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        IrExpression::VarRef(name) => name.to_case(Case::Camel),
        IrExpression::LocalRef(name) => name.to_case(Case::Camel),
        IrExpression::ResourceRef {
            resource_type: _,
            name,
            attribute,
        } => {
            let var_name = name.to_case(Case::Camel);
            match attribute {
                Some(attr) => format!("{}.{}", var_name, attr.to_case(Case::Camel)),
                None => var_name,
            }
        }
        IrExpression::DataRef {
            data_type: _,
            name,
            attribute,
        } => {
            let var_name = name.to_case(Case::Camel);
            match attribute {
                Some(attr) => format!("{}.{}", var_name, attr.to_case(Case::Camel)),
                None => var_name,
            }
        }
        IrExpression::ModuleRef { name, output } => {
            format!("{}.{}", name.to_case(Case::Camel), output.to_case(Case::Camel))
        }
        IrExpression::EachRef(key) => format!("each.{}", key),
        IrExpression::CountIndex => "i".to_string(),
        IrExpression::SelfRef(attr) => format!("this.{}", attr.to_case(Case::Camel)),
        IrExpression::PathRef(kind) => match kind.as_str() {
            "module" => "__dirname".to_string(),
            "root" => "process.cwd()".to_string(),
            "cwd" => "process.cwd()".to_string(),
            _ => format!("/* path.{} */", kind),
        },
        IrExpression::TerraformWorkspace => "devmer.getStack()".to_string(),
        IrExpression::FunctionCall { name, args } => {
            let ts_func = map_function_to_ts(name);
            let args_str: Vec<String> = args.iter().map(expr_to_ts).collect();
            format!("{}({})", ts_func, args_str.join(", "))
        }
        IrExpression::Conditional {
            condition,
            true_result,
            false_result,
        } => {
            format!(
                "{} ? {} : {}",
                expr_to_ts(condition),
                expr_to_ts(true_result),
                expr_to_ts(false_result)
            )
        }
        IrExpression::ForExpr {
            key_var: _,
            value_var,
            collection,
            key_expr: _,
            value_expr,
            condition,
            is_object,
        } => {
            let coll = expr_to_ts(collection);
            let val = expr_to_ts(value_expr);

            if *is_object {
                format!(
                    "Object.fromEntries({}.map({} => [/* key */, {}]){})",
                    coll,
                    value_var,
                    val,
                    condition
                        .as_ref()
                        .map(|c| format!(".filter({} => {})", value_var, expr_to_ts(c)))
                        .unwrap_or_default()
                )
            } else {
                format!(
                    "{}.map({} => {}){}",
                    coll,
                    value_var,
                    val,
                    condition
                        .as_ref()
                        .map(|c| format!(".filter({} => {})", value_var, expr_to_ts(c)))
                        .unwrap_or_default()
                )
            }
        }
        IrExpression::Splat { expr, attribute } => {
            format!("{}.map(x => x.{})", expr_to_ts(expr), attribute.to_case(Case::Camel))
        }
        IrExpression::Index { expr, index } => {
            format!("{}[{}]", expr_to_ts(expr), expr_to_ts(index))
        }
        IrExpression::GetAttr { expr, attr } => {
            format!("{}.{}", expr_to_ts(expr), attr.to_case(Case::Camel))
        }
        IrExpression::BinaryOp { left, op, right } => {
            format!(
                "({} {} {})",
                expr_to_ts(left),
                op.to_string(crate::codegen::Language::TypeScript),
                expr_to_ts(right)
            )
        }
        IrExpression::UnaryOp { op, expr } => {
            let op_str = match op {
                UnaryOperator::Not => "!",
                UnaryOperator::Neg => "-",
            };
            format!("{}{}", op_str, expr_to_ts(expr))
        }
        IrExpression::Template(parts) => {
            let mut result = String::from("`");
            for part in parts {
                match part {
                    TemplatePart::Literal(s) => result.push_str(&escape_template_string(s)),
                    TemplatePart::Interpolation(expr) => {
                        result.push_str(&format!("${{{}}}", expr_to_ts(expr)));
                    }
                    TemplatePart::Directive(d) => {
                        result.push_str(&format!("/* directive: {} */", d));
                    }
                }
            }
            result.push('`');
            result
        }
        IrExpression::Heredoc { content, .. } => {
            format!("`{}`", escape_template_string(content))
        }
        IrExpression::Raw(s) => format!("/* raw: {} */", s),
    }
}

/// Map Terraform function to TypeScript equivalent
fn map_function_to_ts(name: &str) -> String {
    match name {
        "concat" => "Array.prototype.concat.call".to_string(),
        "join" => ".join".to_string(),
        "split" => ".split".to_string(),
        "length" => ".length".to_string(),
        "lower" => ".toLowerCase()".to_string(),
        "upper" => ".toUpperCase()".to_string(),
        "trim" => ".trim()".to_string(),
        "replace" => ".replace".to_string(),
        "format" => "devmer.interpolate".to_string(),
        "jsonencode" => "JSON.stringify".to_string(),
        "jsondecode" => "JSON.parse".to_string(),
        "tolist" => "Array.from".to_string(),
        "toset" => "new Set".to_string(),
        "tomap" => "Object.fromEntries".to_string(),
        "lookup" => "devmer.lookup".to_string(),
        "element" => "devmer.element".to_string(),
        "coalesce" => "devmer.coalesce".to_string(),
        "coalescelist" => "devmer.coalesceList".to_string(),
        "file" => "devmer.fileAsset".to_string(),
        "base64encode" => "Buffer.from".to_string(),
        "base64decode" => "Buffer.from".to_string(),
        "md5" => "devmer.md5".to_string(),
        "sha256" => "devmer.sha256".to_string(),
        "uuid" => "devmer.uuid".to_string(),
        "timestamp" => "new Date().toISOString".to_string(),
        "merge" => "Object.assign".to_string(),
        "keys" => "Object.keys".to_string(),
        "values" => "Object.values".to_string(),
        "flatten" => ".flat()".to_string(),
        "distinct" => "[...new Set".to_string(),
        "sort" => ".sort()".to_string(),
        "reverse" => ".reverse()".to_string(),
        "contains" => ".includes".to_string(),
        "index" => ".indexOf".to_string(),
        "range" => "devmer.range".to_string(),
        "zipmap" => "devmer.zipMap".to_string(),
        "try" => "devmer.try".to_string(),
        "can" => "devmer.can".to_string(),
        _ => format!("devmer.{}", name.to_case(Case::Camel)),
    }
}

/// Escape a string for TypeScript
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a template string for TypeScript
fn escape_template_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

/// Generate types.ts
fn generate_types(module: &IrModule) -> String {
    let mut output = String::from("// Generated types for configuration\n\n");

    for var in &module.variables {
        if let Some(ref var_type) = var.var_type {
            output.push_str(&format!(
                "export type {} = {};\n",
                var.name.to_case(Case::Pascal),
                var_type.to_typescript()
            ));
        }
    }

    output
}

/// Generate package.json
fn generate_package_json(module: &IrModule, options: &ConvertOptions) -> String {
    let project_name = options
        .project_name
        .as_deref()
        .unwrap_or("devmer-project");

    let mut deps = vec!["\"@devmer/sdk\": \"^0.1.0\"".to_string()];

    // Add provider dependencies
    let providers: std::collections::HashSet<&str> = module
        .resources
        .iter()
        .map(|r| r.provider_name())
        .collect();

    for provider in providers {
        if !provider.is_empty() {
            deps.push(format!("\"@devmer/{}\": \"^0.1.0\"", provider));
        }
    }

    format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "tsc",
    "preview": "devmer preview",
    "deploy": "devmer up"
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }},
  "dependencies": {{
    {}
  }}
}}
"#,
        project_name,
        deps.join(",\n    ")
    )
}

/// Generate tsconfig.json
fn generate_tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "./dist"
  },
  "include": ["*.ts"],
  "exclude": ["node_modules"]
}
"#
    .to_string()
}
