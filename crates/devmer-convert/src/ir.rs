//! Intermediate representation for converted HCL

use crate::codegen::Language;
use crate::error::Result;
use crate::ConvertOptions;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// A converted project
#[derive(Debug, Clone)]
pub struct ConvertedProject {
    /// Source directory
    pub source_dir: PathBuf,

    /// Target language
    pub target_language: Language,

    /// Parsed IR module
    pub module: IrModule,

    /// Generated files
    pub generated_files: Vec<crate::codegen::GeneratedFile>,

    /// Conversion options
    pub options: ConvertOptions,
}

impl ConvertedProject {
    /// Write all generated files to disk
    pub fn write_files(&self) -> Result<Vec<PathBuf>> {
        let output_dir = self
            .options
            .output_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(&output_dir)?;

        let mut written = Vec::new();

        for file in &self.generated_files {
            let path = output_dir.join(&file.path);

            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&path, &file.content)?;
            info!("Wrote {}", path.display());
            written.push(path);
        }

        Ok(written)
    }

    /// Get conversion statistics
    pub fn stats(&self) -> crate::ConversionStats {
        crate::ConversionStats::from_module(&self.module)
    }

    /// Get the main entry file
    pub fn main_file(&self) -> Option<&crate::codegen::GeneratedFile> {
        self.generated_files.iter().find(|f| f.is_main)
    }

    /// Preview what files would be written (dry run)
    pub fn preview(&self) -> Vec<(PathBuf, usize)> {
        let output_dir = self
            .options
            .output_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        self.generated_files
            .iter()
            .map(|f| (output_dir.join(&f.path), f.content.len()))
            .collect()
    }
}

/// An IR module representing a complete HCL project
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrModule {
    /// Terraform/OpenTofu settings
    pub terraform_settings: Option<TerraformSettings>,

    /// Required providers
    pub required_providers: IndexMap<String, ProviderRequirement>,

    /// Provider configurations
    pub providers: Vec<IrProvider>,

    /// Variables (inputs)
    pub variables: Vec<IrVariable>,

    /// Local values
    pub locals: IndexMap<String, IrExpression>,

    /// Resources
    pub resources: Vec<IrResource>,

    /// Data sources
    pub data_sources: Vec<IrDataSource>,

    /// Outputs
    pub outputs: Vec<IrOutput>,

    /// Modules
    pub modules: Vec<IrModuleCall>,

    /// Source files processed
    pub source_files: Vec<PathBuf>,
}

/// Terraform settings block
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TerraformSettings {
    /// Required Terraform version
    pub required_version: Option<String>,

    /// Backend configuration
    pub backend: Option<BackendConfig>,

    /// Cloud configuration
    pub cloud: Option<CloudConfig>,
}

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Backend type
    pub backend_type: String,

    /// Backend attributes
    pub attributes: IndexMap<String, IrExpression>,
}

/// Cloud configuration (Terraform Cloud)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    /// Organization
    pub organization: Option<String>,

    /// Workspaces
    pub workspaces: Option<IndexMap<String, IrExpression>>,
}

/// Provider requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequirement {
    /// Source (e.g., "hashicorp/aws")
    pub source: Option<String>,

    /// Version constraint
    pub version: Option<String>,
}

/// A generic IR block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrBlock {
    /// Block type
    pub block_type: String,

    /// Block labels
    pub labels: Vec<String>,

    /// Block attributes
    pub attributes: IndexMap<String, IrExpression>,

    /// Nested blocks
    pub blocks: Vec<IrBlock>,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrProvider {
    /// Provider name (e.g., "aws")
    pub name: String,

    /// Alias (for multiple provider configs)
    pub alias: Option<String>,

    /// Configuration attributes
    pub config: IndexMap<String, IrExpression>,

    /// Original comment
    pub comment: Option<String>,
}

/// Variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrVariable {
    /// Variable name
    pub name: String,

    /// Type constraint
    pub var_type: Option<IrType>,

    /// Default value
    pub default: Option<IrExpression>,

    /// Description
    pub description: Option<String>,

    /// Sensitive flag
    pub sensitive: bool,

    /// Nullable flag
    pub nullable: bool,

    /// Validation rules
    pub validations: Vec<IrValidation>,

    /// Original comment
    pub comment: Option<String>,
}

/// Type representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrType {
    String,
    Number,
    Bool,
    List(Box<IrType>),
    Set(Box<IrType>),
    Map(Box<IrType>),
    Object(IndexMap<String, IrType>),
    Tuple(Vec<IrType>),
    Any,
}

impl IrType {
    /// Convert to TypeScript type
    pub fn to_typescript(&self) -> String {
        match self {
            IrType::String => "string".to_string(),
            IrType::Number => "number".to_string(),
            IrType::Bool => "boolean".to_string(),
            IrType::List(inner) => format!("{}[]", inner.to_typescript()),
            IrType::Set(inner) => format!("Set<{}>", inner.to_typescript()),
            IrType::Map(inner) => format!("Record<string, {}>", inner.to_typescript()),
            IrType::Object(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_typescript()))
                    .collect();
                format!("{{ {} }}", fields_str.join(", "))
            }
            IrType::Tuple(types) => {
                let types_str: Vec<String> = types.iter().map(|t| t.to_typescript()).collect();
                format!("[{}]", types_str.join(", "))
            }
            IrType::Any => "any".to_string(),
        }
    }

    /// Convert to Python type hint
    pub fn to_python(&self) -> String {
        match self {
            IrType::String => "str".to_string(),
            IrType::Number => "float".to_string(),
            IrType::Bool => "bool".to_string(),
            IrType::List(inner) => format!("list[{}]", inner.to_python()),
            IrType::Set(inner) => format!("set[{}]", inner.to_python()),
            IrType::Map(inner) => format!("dict[str, {}]", inner.to_python()),
            IrType::Object(fields) => {
                // TypedDict would be better but this works
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_python()))
                    .collect();
                format!("dict[str, Any]  # {{ {} }}", fields_str.join(", "))
            }
            IrType::Tuple(types) => {
                let types_str: Vec<String> = types.iter().map(|t| t.to_python()).collect();
                format!("tuple[{}]", types_str.join(", "))
            }
            IrType::Any => "Any".to_string(),
        }
    }

    /// Convert to Go type
    pub fn to_go(&self) -> String {
        match self {
            IrType::String => "string".to_string(),
            IrType::Number => "float64".to_string(),
            IrType::Bool => "bool".to_string(),
            IrType::List(inner) => format!("[]{}", inner.to_go()),
            IrType::Set(inner) => format!("[]{}", inner.to_go()), // Go doesn't have sets
            IrType::Map(inner) => format!("map[string]{}", inner.to_go()),
            IrType::Object(_) => "map[string]interface{}".to_string(),
            IrType::Tuple(_) => "[]interface{}".to_string(),
            IrType::Any => "interface{}".to_string(),
        }
    }
}

/// Validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrValidation {
    /// Condition expression
    pub condition: IrExpression,

    /// Error message
    pub error_message: String,
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrResource {
    /// Resource type (e.g., "aws_s3_bucket")
    pub resource_type: String,

    /// Resource name
    pub name: String,

    /// Provider alias
    pub provider: Option<String>,

    /// Attributes
    pub attributes: IndexMap<String, IrExpression>,

    /// Nested blocks
    pub blocks: Vec<IrBlock>,

    /// Depends on
    pub depends_on: Vec<String>,

    /// Count expression
    pub count: Option<IrExpression>,

    /// For-each expression
    pub for_each: Option<IrExpression>,

    /// Lifecycle settings
    pub lifecycle: Option<IrLifecycle>,

    /// Original comment
    pub comment: Option<String>,
}

impl IrResource {
    /// Get the provider name from resource type
    pub fn provider_name(&self) -> &str {
        self.resource_type
            .split('_')
            .next()
            .unwrap_or(&self.resource_type)
    }

    /// Get the resource type without provider prefix
    pub fn type_without_provider(&self) -> String {
        let parts: Vec<&str> = self.resource_type.splitn(2, '_').collect();
        if parts.len() > 1 {
            parts[1].to_string()
        } else {
            self.resource_type.clone()
        }
    }
}

/// Lifecycle settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrLifecycle {
    /// Create before destroy
    pub create_before_destroy: bool,

    /// Prevent destroy
    pub prevent_destroy: bool,

    /// Ignore changes
    pub ignore_changes: Vec<String>,

    /// Replace triggered by
    pub replace_triggered_by: Vec<String>,
}

/// Data source definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrDataSource {
    /// Data source type
    pub data_type: String,

    /// Data source name
    pub name: String,

    /// Provider alias
    pub provider: Option<String>,

    /// Attributes
    pub attributes: IndexMap<String, IrExpression>,

    /// Nested blocks
    pub blocks: Vec<IrBlock>,

    /// Original comment
    pub comment: Option<String>,
}

/// Output definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrOutput {
    /// Output name
    pub name: String,

    /// Value expression
    pub value: IrExpression,

    /// Description
    pub description: Option<String>,

    /// Sensitive flag
    pub sensitive: bool,

    /// Depends on
    pub depends_on: Vec<String>,

    /// Original comment
    pub comment: Option<String>,
}

/// Module call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrModuleCall {
    /// Module name
    pub name: String,

    /// Source
    pub source: String,

    /// Version
    pub version: Option<String>,

    /// Input values
    pub inputs: IndexMap<String, IrExpression>,

    /// Providers mapping
    pub providers: IndexMap<String, String>,

    /// Depends on
    pub depends_on: Vec<String>,

    /// Count expression
    pub count: Option<IrExpression>,

    /// For-each expression
    pub for_each: Option<IrExpression>,

    /// Original comment
    pub comment: Option<String>,
}

/// Expression representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrExpression {
    /// Literal null
    Null,

    /// Boolean literal
    Bool(bool),

    /// Number literal
    Number(f64),

    /// String literal
    String(String),

    /// List/tuple literal
    List(Vec<IrExpression>),

    /// Object/map literal
    Object(IndexMap<String, IrExpression>),

    /// Variable reference (var.name)
    VarRef(String),

    /// Local reference (local.name)
    LocalRef(String),

    /// Resource reference (resource_type.name.attribute)
    ResourceRef {
        resource_type: String,
        name: String,
        attribute: Option<String>,
    },

    /// Data source reference (data.type.name.attribute)
    DataRef {
        data_type: String,
        name: String,
        attribute: Option<String>,
    },

    /// Module output reference (module.name.output)
    ModuleRef { name: String, output: String },

    /// Each reference (each.key, each.value)
    EachRef(String),

    /// Count reference (count.index)
    CountIndex,

    /// Self reference (self.attribute)
    SelfRef(String),

    /// Path reference (path.module, path.root, path.cwd)
    PathRef(String),

    /// Terraform workspace
    TerraformWorkspace,

    /// Function call
    FunctionCall {
        name: String,
        args: Vec<IrExpression>,
    },

    /// Conditional expression (condition ? true_val : false_val)
    Conditional {
        condition: Box<IrExpression>,
        true_result: Box<IrExpression>,
        false_result: Box<IrExpression>,
    },

    /// For expression
    ForExpr {
        key_var: Option<String>,
        value_var: String,
        collection: Box<IrExpression>,
        key_expr: Option<Box<IrExpression>>,
        value_expr: Box<IrExpression>,
        condition: Option<Box<IrExpression>>,
        is_object: bool,
    },

    /// Splat expression (resource[*].attribute)
    Splat {
        expr: Box<IrExpression>,
        attribute: String,
    },

    /// Index access (expr[index])
    Index {
        expr: Box<IrExpression>,
        index: Box<IrExpression>,
    },

    /// Attribute access (expr.attr)
    GetAttr {
        expr: Box<IrExpression>,
        attr: String,
    },

    /// Binary operation
    BinaryOp {
        left: Box<IrExpression>,
        op: BinaryOperator,
        right: Box<IrExpression>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        expr: Box<IrExpression>,
    },

    /// Template string with interpolations
    Template(Vec<TemplatePart>),

    /// Heredoc
    Heredoc {
        delimiter: String,
        content: String,
        strip_indent: bool,
    },

    /// Raw HCL that couldn't be parsed (fallback)
    Raw(String),
}

/// Binary operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinaryOperator {
    /// Convert to language-specific operator string
    pub fn to_string(&self, lang: Language) -> &'static str {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Mod => "%",
            BinaryOperator::Eq => match lang {
                Language::TypeScript | Language::Go => "===",
                Language::Python | Language::Rhai => "==",
            },
            BinaryOperator::Ne => match lang {
                Language::TypeScript | Language::Go => "!==",
                Language::Python | Language::Rhai => "!=",
            },
            BinaryOperator::Lt => "<",
            BinaryOperator::Le => "<=",
            BinaryOperator::Gt => ">",
            BinaryOperator::Ge => ">=",
            BinaryOperator::And => match lang {
                Language::Python => "and",
                _ => "&&",
            },
            BinaryOperator::Or => match lang {
                Language::Python => "or",
                _ => "||",
            },
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Neg,
}

/// Template part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplatePart {
    /// Literal text
    Literal(String),
    /// Interpolation
    Interpolation(Box<IrExpression>),
    /// Directive (if, for, etc.)
    Directive(String),
}
