//! HCL file parser

use crate::error::{ConvertError, Result};
use crate::hcl::expressions::parse_expression;
use crate::ir::*;
use hcl::{Block, Body, Expression, Structure};
use indexmap::IndexMap;
use std::path::Path;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// HCL Parser for Terraform/OpenTofu files
pub struct HclParser {
    /// Whether to include comments
    include_comments: bool,
}

impl HclParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            include_comments: true,
        }
    }

    /// Disable comment parsing
    pub fn without_comments(mut self) -> Self {
        self.include_comments = false;
        self
    }

    /// Parse a directory of HCL files
    pub fn parse_directory(&self, dir: &Path) -> Result<IrModule> {
        if !dir.exists() {
            return Err(ConvertError::FileNotFound(dir.to_path_buf()));
        }

        let mut module = IrModule::default();
        let mut found_files = false;

        for entry in WalkDir::new(dir)
            .max_depth(1) // Don't recurse into modules
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|e| e.to_str());

                match extension {
                    Some("tf") => {
                        info!("Parsing HCL file: {}", path.display());
                        self.parse_file_into(&path, &mut module)?;
                        module.source_files.push(path.to_path_buf());
                        found_files = true;
                    }
                    Some("json") if path.to_string_lossy().ends_with(".tf.json") => {
                        info!("Parsing HCL JSON file: {}", path.display());
                        self.parse_json_file_into(&path, &mut module)?;
                        module.source_files.push(path.to_path_buf());
                        found_files = true;
                    }
                    _ => {}
                }
            }
        }

        if !found_files {
            return Err(ConvertError::NoHclFiles(dir.to_path_buf()));
        }

        Ok(module)
    }

    /// Parse a single HCL file into an existing module
    fn parse_file_into(&self, path: &Path, module: &mut IrModule) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let body: Body = hcl::from_str(&content)
            .map_err(|e| ConvertError::parse_error(path, e.to_string()))?;

        self.process_body(&body, module, path)?;
        Ok(())
    }

    /// Parse a JSON HCL file
    fn parse_json_file_into(&self, path: &Path, module: &mut IrModule) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let body: Body = hcl::from_str(&content)
            .map_err(|e| ConvertError::parse_error(path, e.to_string()))?;

        self.process_body(&body, module, path)?;
        Ok(())
    }

    /// Process an HCL body
    fn process_body(&self, body: &Body, module: &mut IrModule, path: &Path) -> Result<()> {
        for structure in body.iter() {
            match structure {
                Structure::Block(block) => {
                    self.process_block(block, module, path)?;
                }
                Structure::Attribute(attr) => {
                    debug!("Top-level attribute: {}", attr.key);
                }
            }
        }
        Ok(())
    }

    /// Process a block
    fn process_block(&self, block: &Block, module: &mut IrModule, _path: &Path) -> Result<()> {
        let block_type = block.identifier.as_str();
        let labels: Vec<&str> = block.labels.iter().map(|l| l.as_str()).collect();

        match block_type {
            "terraform" => {
                self.process_terraform_block(block, module)?;
            }
            "provider" => {
                if let Some(provider) = self.process_provider_block(block, &labels)? {
                    module.providers.push(provider);
                }
            }
            "variable" => {
                if let Some(var) = self.process_variable_block(block, &labels)? {
                    module.variables.push(var);
                }
            }
            "locals" => {
                self.process_locals_block(block, module)?;
            }
            "resource" => {
                if let Some(resource) = self.process_resource_block(block, &labels)? {
                    module.resources.push(resource);
                }
            }
            "data" => {
                if let Some(data) = self.process_data_block(block, &labels)? {
                    module.data_sources.push(data);
                }
            }
            "output" => {
                if let Some(output) = self.process_output_block(block, &labels)? {
                    module.outputs.push(output);
                }
            }
            "module" => {
                if let Some(mod_call) = self.process_module_block(block, &labels)? {
                    module.modules.push(mod_call);
                }
            }
            _ => {
                warn!("Unknown block type: {}", block_type);
            }
        }

        Ok(())
    }

    /// Process terraform settings block
    fn process_terraform_block(&self, block: &Block, module: &mut IrModule) -> Result<()> {
        let mut settings = TerraformSettings::default();

        for structure in block.body.iter() {
            match structure {
                Structure::Attribute(attr) => {
                    if attr.key.as_str() == "required_version" {
                        if let Some(s) = self.expr_to_string(&attr.expr) {
                            settings.required_version = Some(s);
                        }
                    }
                }
                Structure::Block(inner) => {
                    let inner_type = inner.identifier.as_str();
                    match inner_type {
                        "required_providers" => {
                            self.process_required_providers(inner, module)?;
                        }
                        "backend" => {
                            if let Some(backend) = self.process_backend_block(inner)? {
                                settings.backend = Some(backend);
                            }
                        }
                        "cloud" => {
                            // Process cloud block
                        }
                        _ => {}
                    }
                }
            }
        }

        module.terraform_settings = Some(settings);
        Ok(())
    }

    /// Process required_providers block
    fn process_required_providers(&self, block: &Block, module: &mut IrModule) -> Result<()> {
        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                let provider_name = attr.key.as_str().to_string();
                let mut requirement = ProviderRequirement {
                    source: None,
                    version: None,
                };

                if let Expression::Object(obj) = &attr.expr {
                    for (key, value) in obj.iter() {
                        let key_str = match key {
                            hcl::ObjectKey::Identifier(id) => id.as_str(),
                            hcl::ObjectKey::Expression(_) => continue,
                            _ => continue,
                        };
                        match key_str {
                            "source" => {
                                requirement.source = self.expr_to_string(value);
                            }
                            "version" => {
                                requirement.version = self.expr_to_string(value);
                            }
                            _ => {}
                        }
                    }
                }

                module.required_providers.insert(provider_name, requirement);
            }
        }
        Ok(())
    }

    /// Process backend block
    fn process_backend_block(&self, block: &Block) -> Result<Option<BackendConfig>> {
        let backend_type = block.labels.first().map(|l| l.as_str().to_string());

        if let Some(backend_type) = backend_type {
            let attributes = self.process_attributes(&block.body)?;
            Ok(Some(BackendConfig {
                backend_type,
                attributes,
            }))
        } else {
            Ok(None)
        }
    }

    /// Process provider block
    fn process_provider_block(
        &self,
        block: &Block,
        labels: &[&str],
    ) -> Result<Option<IrProvider>> {
        let name = labels.first().map(|s| s.to_string()).unwrap_or_default();

        if name.is_empty() {
            return Ok(None);
        }

        let mut provider = IrProvider {
            name,
            alias: None,
            config: IndexMap::new(),
            comment: None,
        };

        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                let key = attr.key.as_str();
                if key == "alias" {
                    provider.alias = self.expr_to_string(&attr.expr);
                } else {
                    provider.config.insert(
                        key.to_string(),
                        parse_expression(&attr.expr),
                    );
                }
            }
        }

        Ok(Some(provider))
    }

    /// Process variable block
    fn process_variable_block(
        &self,
        block: &Block,
        labels: &[&str],
    ) -> Result<Option<IrVariable>> {
        let name = labels.first().map(|s| s.to_string()).unwrap_or_default();

        if name.is_empty() {
            return Ok(None);
        }

        let mut var = IrVariable {
            name,
            var_type: None,
            default: None,
            description: None,
            sensitive: false,
            nullable: true,
            validations: vec![],
            comment: None,
        };

        for structure in block.body.iter() {
            match structure {
                Structure::Attribute(attr) => {
                    let key = attr.key.as_str();
                    match key {
                        "type" => {
                            var.var_type = self.parse_type_expr(&attr.expr);
                        }
                        "default" => {
                            var.default = Some(parse_expression(&attr.expr));
                        }
                        "description" => {
                            var.description = self.expr_to_string(&attr.expr);
                        }
                        "sensitive" => {
                            var.sensitive = self.expr_to_bool(&attr.expr).unwrap_or(false);
                        }
                        "nullable" => {
                            var.nullable = self.expr_to_bool(&attr.expr).unwrap_or(true);
                        }
                        _ => {}
                    }
                }
                Structure::Block(inner) if inner.identifier.as_str() == "validation" => {
                    if let Some(validation) = self.process_validation_block(inner)? {
                        var.validations.push(validation);
                    }
                }
                _ => {}
            }
        }

        Ok(Some(var))
    }

    /// Process validation block
    fn process_validation_block(&self, block: &Block) -> Result<Option<IrValidation>> {
        let mut condition = None;
        let mut error_message = String::new();

        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                match attr.key.as_str() {
                    "condition" => {
                        condition = Some(parse_expression(&attr.expr));
                    }
                    "error_message" => {
                        error_message = self.expr_to_string(&attr.expr).unwrap_or_default();
                    }
                    _ => {}
                }
            }
        }

        if let Some(condition) = condition {
            Ok(Some(IrValidation {
                condition,
                error_message,
            }))
        } else {
            Ok(None)
        }
    }

    /// Process locals block
    fn process_locals_block(&self, block: &Block, module: &mut IrModule) -> Result<()> {
        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                let name = attr.key.as_str().to_string();
                let value = parse_expression(&attr.expr);
                module.locals.insert(name, value);
            }
        }
        Ok(())
    }

    /// Process resource block
    fn process_resource_block(
        &self,
        block: &Block,
        labels: &[&str],
    ) -> Result<Option<IrResource>> {
        if labels.len() < 2 {
            return Ok(None);
        }

        let resource_type = labels[0].to_string();
        let name = labels[1].to_string();

        let mut resource = IrResource {
            resource_type,
            name,
            provider: None,
            attributes: IndexMap::new(),
            blocks: vec![],
            depends_on: vec![],
            count: None,
            for_each: None,
            lifecycle: None,
            comment: None,
        };

        for structure in block.body.iter() {
            match structure {
                Structure::Attribute(attr) => {
                    let key = attr.key.as_str();
                    match key {
                        "provider" => {
                            resource.provider = self.expr_to_string(&attr.expr);
                        }
                        "depends_on" => {
                            resource.depends_on = self.expr_to_string_list(&attr.expr);
                        }
                        "count" => {
                            resource.count = Some(parse_expression(&attr.expr));
                        }
                        "for_each" => {
                            resource.for_each = Some(parse_expression(&attr.expr));
                        }
                        _ => {
                            resource.attributes.insert(
                                key.to_string(),
                                parse_expression(&attr.expr),
                            );
                        }
                    }
                }
                Structure::Block(inner) => {
                    if inner.identifier.as_str() == "lifecycle" {
                        resource.lifecycle = self.process_lifecycle_block(inner)?;
                    } else {
                        resource.blocks.push(self.process_nested_block(inner)?);
                    }
                }
            }
        }

        Ok(Some(resource))
    }

    /// Process lifecycle block
    fn process_lifecycle_block(&self, block: &Block) -> Result<Option<IrLifecycle>> {
        let mut lifecycle = IrLifecycle::default();

        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                match attr.key.as_str() {
                    "create_before_destroy" => {
                        lifecycle.create_before_destroy =
                            self.expr_to_bool(&attr.expr).unwrap_or(false);
                    }
                    "prevent_destroy" => {
                        lifecycle.prevent_destroy =
                            self.expr_to_bool(&attr.expr).unwrap_or(false);
                    }
                    "ignore_changes" => {
                        lifecycle.ignore_changes = self.expr_to_string_list(&attr.expr);
                    }
                    "replace_triggered_by" => {
                        lifecycle.replace_triggered_by = self.expr_to_string_list(&attr.expr);
                    }
                    _ => {}
                }
            }
        }

        Ok(Some(lifecycle))
    }

    /// Process nested block
    fn process_nested_block(&self, block: &Block) -> Result<IrBlock> {
        let block_type = block.identifier.as_str().to_string();
        let labels: Vec<String> = block.labels.iter().map(|l| l.as_str().to_string()).collect();

        let attributes = self.process_attributes(&block.body)?;
        let mut blocks = vec![];

        for structure in block.body.iter() {
            if let Structure::Block(inner) = structure {
                blocks.push(self.process_nested_block(inner)?);
            }
        }

        Ok(IrBlock {
            block_type,
            labels,
            attributes,
            blocks,
        })
    }

    /// Process data source block
    fn process_data_block(
        &self,
        block: &Block,
        labels: &[&str],
    ) -> Result<Option<IrDataSource>> {
        if labels.len() < 2 {
            return Ok(None);
        }

        let data_type = labels[0].to_string();
        let name = labels[1].to_string();

        let attributes = self.process_attributes(&block.body)?;
        let mut blocks = vec![];

        for structure in block.body.iter() {
            if let Structure::Block(inner) = structure {
                blocks.push(self.process_nested_block(inner)?);
            }
        }

        Ok(Some(IrDataSource {
            data_type,
            name,
            provider: None,
            attributes,
            blocks,
            comment: None,
        }))
    }

    /// Process output block
    fn process_output_block(
        &self,
        block: &Block,
        labels: &[&str],
    ) -> Result<Option<IrOutput>> {
        let name = labels.first().map(|s| s.to_string()).unwrap_or_default();

        if name.is_empty() {
            return Ok(None);
        }

        let mut output = IrOutput {
            name,
            value: IrExpression::Null,
            description: None,
            sensitive: false,
            depends_on: vec![],
            comment: None,
        };

        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                match attr.key.as_str() {
                    "value" => {
                        output.value = parse_expression(&attr.expr);
                    }
                    "description" => {
                        output.description = self.expr_to_string(&attr.expr);
                    }
                    "sensitive" => {
                        output.sensitive = self.expr_to_bool(&attr.expr).unwrap_or(false);
                    }
                    "depends_on" => {
                        output.depends_on = self.expr_to_string_list(&attr.expr);
                    }
                    _ => {}
                }
            }
        }

        Ok(Some(output))
    }

    /// Process module block
    fn process_module_block(
        &self,
        block: &Block,
        labels: &[&str],
    ) -> Result<Option<IrModuleCall>> {
        let name = labels.first().map(|s| s.to_string()).unwrap_or_default();

        if name.is_empty() {
            return Ok(None);
        }

        let mut mod_call = IrModuleCall {
            name,
            source: String::new(),
            version: None,
            inputs: IndexMap::new(),
            providers: IndexMap::new(),
            depends_on: vec![],
            count: None,
            for_each: None,
            comment: None,
        };

        for structure in block.body.iter() {
            if let Structure::Attribute(attr) = structure {
                let key = attr.key.as_str();
                match key {
                    "source" => {
                        mod_call.source = self.expr_to_string(&attr.expr).unwrap_or_default();
                    }
                    "version" => {
                        mod_call.version = self.expr_to_string(&attr.expr);
                    }
                    "depends_on" => {
                        mod_call.depends_on = self.expr_to_string_list(&attr.expr);
                    }
                    "count" => {
                        mod_call.count = Some(parse_expression(&attr.expr));
                    }
                    "for_each" => {
                        mod_call.for_each = Some(parse_expression(&attr.expr));
                    }
                    "providers" => {
                        // Process providers map
                    }
                    _ => {
                        mod_call.inputs.insert(
                            key.to_string(),
                            parse_expression(&attr.expr),
                        );
                    }
                }
            }
        }

        Ok(Some(mod_call))
    }

    /// Process attributes from a body
    fn process_attributes(&self, body: &Body) -> Result<IndexMap<String, IrExpression>> {
        let mut attributes = IndexMap::new();

        for structure in body.iter() {
            if let Structure::Attribute(attr) = structure {
                let key = attr.key.as_str().to_string();
                let value = parse_expression(&attr.expr);
                attributes.insert(key, value);
            }
        }

        Ok(attributes)
    }

    /// Convert expression to string
    fn expr_to_string(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::String(s) => Some(s.to_string()),
            _ => None,
        }
    }

    /// Convert expression to bool
    fn expr_to_bool(&self, expr: &Expression) -> Option<bool> {
        match expr {
            Expression::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert expression to string list
    fn expr_to_string_list(&self, expr: &Expression) -> Vec<String> {
        match expr {
            Expression::Array(arr) => arr
                .iter()
                .filter_map(|e| {
                    if let Expression::String(s) = e {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => vec![],
        }
    }

    /// Parse type expression
    fn parse_type_expr(&self, _expr: &Expression) -> Option<IrType> {
        // Type expressions in HCL are complex, simplified for now
        Some(IrType::Any)
    }
}

impl Default for HclParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_simple_resource() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "main.tf",
            r#"
resource "aws_s3_bucket" "example" {
  bucket = "my-bucket"

  tags = {
    Name = "My Bucket"
  }
}
"#,
        );

        let parser = HclParser::new();
        let module = parser.parse_directory(temp.path()).unwrap();

        assert_eq!(module.resources.len(), 1);
        assert_eq!(module.resources[0].resource_type, "aws_s3_bucket");
        assert_eq!(module.resources[0].name, "example");
    }

    #[test]
    fn test_parse_variable() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "variables.tf",
            r#"
variable "bucket_name" {
  type        = string
  description = "The name of the bucket"
  default     = "default-bucket"
}
"#,
        );

        let parser = HclParser::new();
        let module = parser.parse_directory(temp.path()).unwrap();

        assert_eq!(module.variables.len(), 1);
        assert_eq!(module.variables[0].name, "bucket_name");
        assert_eq!(
            module.variables[0].description,
            Some("The name of the bucket".to_string())
        );
    }

    #[test]
    fn test_parse_output() {
        let temp = TempDir::new().unwrap();
        create_test_file(
            temp.path(),
            "outputs.tf",
            r#"
output "bucket_arn" {
  value       = aws_s3_bucket.example.arn
  description = "The ARN of the bucket"
  sensitive   = true
}
"#,
        );

        let parser = HclParser::new();
        let module = parser.parse_directory(temp.path()).unwrap();

        assert_eq!(module.outputs.len(), 1);
        assert_eq!(module.outputs[0].name, "bucket_arn");
        assert!(module.outputs[0].sensitive);
    }
}
