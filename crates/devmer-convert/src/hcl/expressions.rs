//! HCL expression parsing

use crate::ir::*;
use hcl::Expression;
use indexmap::IndexMap;

/// Parse an HCL expression to IR
pub fn parse_expression(expr: &Expression) -> IrExpression {
    match expr {
        Expression::Null => IrExpression::Null,
        Expression::Bool(b) => IrExpression::Bool(*b),
        Expression::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            IrExpression::Number(f)
        }
        Expression::String(s) => {
            // Check for interpolations
            let s_str = s.to_string();
            if s_str.contains("${") || s_str.contains("%{") {
                parse_template_string(&s_str)
            } else {
                IrExpression::String(s_str)
            }
        }
        Expression::Array(arr) => {
            let items: Vec<IrExpression> = arr.iter().map(parse_expression).collect();
            IrExpression::List(items)
        }
        Expression::Object(obj) => {
            let mut map = IndexMap::new();
            for (key, value) in obj.iter() {
                let key_str = match key {
                    hcl::expr::ObjectKey::Identifier(id) => id.as_str().to_string(),
                    hcl::expr::ObjectKey::Expression(expr) => {
                        // For expression keys, convert to string
                        format!("{:?}", expr)
                    }
                    _ => continue, // Non-exhaustive enum
                };
                map.insert(key_str, parse_expression(value));
            }
            IrExpression::Object(map)
        }
        Expression::TemplateExpr(template) => {
            // Template expressions like "Hello ${var.name}!"
            let content = template.to_string();
            parse_template_string(&content)
        }
        Expression::Variable(var) => {
            let var_str = var.as_str();
            parse_variable_reference(var_str)
        }
        Expression::Traversal(traversal) => parse_traversal(traversal),
        Expression::FuncCall(func_call) => {
            let name = func_call.name.name.as_str().to_string();
            let args: Vec<IrExpression> = func_call.args.iter().map(parse_expression).collect();
            IrExpression::FunctionCall { name, args }
        }
        Expression::Conditional(cond) => IrExpression::Conditional {
            condition: Box::new(parse_expression(&cond.cond_expr)),
            true_result: Box::new(parse_expression(&cond.true_expr)),
            false_result: Box::new(parse_expression(&cond.false_expr)),
        },
        Expression::Operation(op) => parse_operation(op),
        Expression::ForExpr(for_expr) => {
            let key_var = for_expr.key_var.as_ref().map(|k| k.as_str().to_string());
            let value_var = for_expr.value_var.as_str().to_string();
            let collection = Box::new(parse_expression(&for_expr.collection_expr));
            let key_expr = for_expr
                .key_expr
                .as_ref()
                .map(|e| Box::new(parse_expression(e)));
            let value_expr = Box::new(parse_expression(&for_expr.value_expr));
            let condition = for_expr
                .cond_expr
                .as_ref()
                .map(|e| Box::new(parse_expression(e)));
            let is_object = for_expr.key_expr.is_some();

            IrExpression::ForExpr {
                key_var,
                value_var,
                collection,
                key_expr,
                value_expr,
                condition,
                is_object,
            }
        }
        Expression::Parenthesis(inner) => parse_expression(inner),
        _ => {
            // Fallback for unsupported expressions
            IrExpression::Raw(format!("{:?}", expr))
        }
    }
}

/// Parse a traversal expression (e.g., aws_s3_bucket.example.arn)
fn parse_traversal(traversal: &hcl::expr::Traversal) -> IrExpression {
    // Get the base expression as a string
    let base = match &traversal.expr {
        Expression::Variable(v) => v.as_str().to_string(),
        other => format!("{:?}", other),
    };

    let parts: Vec<String> = std::iter::once(base)
        .chain(traversal.operators.iter().map(|op| match op {
            hcl::expr::TraversalOperator::GetAttr(ident) => ident.as_str().to_string(),
            hcl::expr::TraversalOperator::Index(expr) => {
                format!("[{:?}]", expr)
            }
            hcl::expr::TraversalOperator::AttrSplat => "[*]".to_string(),
            hcl::expr::TraversalOperator::FullSplat => "[*]".to_string(),
            hcl::expr::TraversalOperator::LegacyIndex(idx) => {
                format!(".{}", idx)
            }
        }))
        .collect();

    parse_dotted_reference(&parts)
}

/// Parse a dotted reference like "var.name" or "aws_s3_bucket.example.arn"
fn parse_dotted_reference(parts: &[String]) -> IrExpression {
    if parts.is_empty() {
        return IrExpression::Null;
    }

    let first = &parts[0];
    let rest: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();

    match first.as_str() {
        "var" => {
            if let Some(name) = rest.first() {
                let mut expr = IrExpression::VarRef(name.to_string());
                for attr in rest.iter().skip(1) {
                    expr = IrExpression::GetAttr {
                        expr: Box::new(expr),
                        attr: attr.to_string(),
                    };
                }
                expr
            } else {
                IrExpression::Raw("var".to_string())
            }
        }
        "local" => {
            if let Some(name) = rest.first() {
                let mut expr = IrExpression::LocalRef(name.to_string());
                for attr in rest.iter().skip(1) {
                    expr = IrExpression::GetAttr {
                        expr: Box::new(expr),
                        attr: attr.to_string(),
                    };
                }
                expr
            } else {
                IrExpression::Raw("local".to_string())
            }
        }
        "data" => {
            if rest.len() >= 2 {
                let data_type = rest[0].to_string();
                let name = rest[1].to_string();
                let attribute = rest.get(2).map(|s| s.to_string());

                let mut expr = IrExpression::DataRef {
                    data_type,
                    name,
                    attribute,
                };

                for attr in rest.iter().skip(3) {
                    expr = IrExpression::GetAttr {
                        expr: Box::new(expr),
                        attr: attr.to_string(),
                    };
                }
                expr
            } else {
                IrExpression::Raw(parts.join("."))
            }
        }
        "module" => {
            if rest.len() >= 2 {
                IrExpression::ModuleRef {
                    name: rest[0].to_string(),
                    output: rest[1].to_string(),
                }
            } else {
                IrExpression::Raw(parts.join("."))
            }
        }
        "each" => {
            if let Some(key) = rest.first() {
                IrExpression::EachRef(key.to_string())
            } else {
                IrExpression::Raw("each".to_string())
            }
        }
        "count" => {
            if rest.first().map(|s| *s) == Some("index") {
                IrExpression::CountIndex
            } else {
                IrExpression::Raw(parts.join("."))
            }
        }
        "self" => {
            if let Some(attr) = rest.first() {
                IrExpression::SelfRef(attr.to_string())
            } else {
                IrExpression::Raw("self".to_string())
            }
        }
        "path" => {
            if let Some(kind) = rest.first() {
                IrExpression::PathRef(kind.to_string())
            } else {
                IrExpression::Raw("path".to_string())
            }
        }
        "terraform" => {
            if rest.first().map(|s| *s) == Some("workspace") {
                IrExpression::TerraformWorkspace
            } else {
                IrExpression::Raw(parts.join("."))
            }
        }
        _ => {
            // Resource reference
            let resource_type = first.clone();
            if let Some(name) = rest.first() {
                let attribute = rest.get(1).map(|s| s.to_string());

                let mut expr = IrExpression::ResourceRef {
                    resource_type,
                    name: name.to_string(),
                    attribute,
                };

                for attr in rest.iter().skip(2) {
                    if attr.starts_with('[') {
                        // Index access
                        expr = IrExpression::Splat {
                            expr: Box::new(expr),
                            attribute: attr.to_string(),
                        };
                    } else {
                        expr = IrExpression::GetAttr {
                            expr: Box::new(expr),
                            attr: attr.to_string(),
                        };
                    }
                }
                expr
            } else {
                IrExpression::Raw(parts.join("."))
            }
        }
    }
}

/// Parse a variable reference string
fn parse_variable_reference(var_str: &str) -> IrExpression {
    let parts: Vec<String> = var_str.split('.').map(|s| s.to_string()).collect();
    parse_dotted_reference(&parts)
}

/// Parse a template string with interpolations
fn parse_template_string(s: &str) -> IrExpression {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' || c == '%' {
            if chars.peek() == Some(&'{') {
                // Start of interpolation
                if !current.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut current)));
                }
                chars.next(); // consume '{'

                let mut depth = 1;
                let mut expr_str = String::new();
                let is_directive = c == '%';

                while depth > 0 {
                    if let Some(ch) = chars.next() {
                        match ch {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            expr_str.push(ch);
                        }
                    } else {
                        break;
                    }
                }

                if is_directive {
                    parts.push(TemplatePart::Directive(expr_str));
                } else {
                    // Parse the interpolation expression
                    let expr = parse_interpolation_expr(&expr_str);
                    parts.push(TemplatePart::Interpolation(Box::new(expr)));
                }
            } else {
                current.push(c);
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        parts.push(TemplatePart::Literal(current));
    }

    if parts.len() == 1 {
        if let TemplatePart::Literal(s) = &parts[0] {
            return IrExpression::String(s.clone());
        }
    }

    IrExpression::Template(parts)
}

/// Parse an interpolation expression (simplified)
fn parse_interpolation_expr(expr_str: &str) -> IrExpression {
    let trimmed = expr_str.trim();

    // Simple variable/reference parsing
    let parts: Vec<String> = trimmed.split('.').map(|s| s.trim().to_string()).collect();

    if parts.is_empty() {
        return IrExpression::Raw(expr_str.to_string());
    }

    parse_dotted_reference(&parts)
}

/// Parse an operation expression
fn parse_operation(op: &hcl::expr::Operation) -> IrExpression {
    match op {
        hcl::expr::Operation::Binary(binary) => {
            let left = Box::new(parse_expression(&binary.lhs_expr));
            let right = Box::new(parse_expression(&binary.rhs_expr));
            let op = convert_binary_op(&binary.operator);

            IrExpression::BinaryOp { left, op, right }
        }
        hcl::expr::Operation::Unary(unary) => {
            let expr = Box::new(parse_expression(&unary.expr));
            let op = convert_unary_op(&unary.operator);

            IrExpression::UnaryOp { op, expr }
        }
    }
}

/// Convert HCL binary operator to IR
fn convert_binary_op(op: &hcl::expr::BinaryOperator) -> BinaryOperator {
    use hcl::expr::BinaryOperator as HclOp;
    match op {
        HclOp::Plus => BinaryOperator::Add,
        HclOp::Minus => BinaryOperator::Sub,
        HclOp::Mul => BinaryOperator::Mul,
        HclOp::Div => BinaryOperator::Div,
        HclOp::Mod => BinaryOperator::Mod,
        HclOp::Eq => BinaryOperator::Eq,
        HclOp::NotEq => BinaryOperator::Ne,
        HclOp::Less => BinaryOperator::Lt,
        HclOp::LessEq => BinaryOperator::Le,
        HclOp::Greater => BinaryOperator::Gt,
        HclOp::GreaterEq => BinaryOperator::Ge,
        HclOp::And => BinaryOperator::And,
        HclOp::Or => BinaryOperator::Or,
    }
}

/// Convert HCL unary operator to IR
fn convert_unary_op(op: &hcl::expr::UnaryOperator) -> UnaryOperator {
    use hcl::expr::UnaryOperator as HclOp;
    match op {
        HclOp::Not => UnaryOperator::Not,
        HclOp::Neg => UnaryOperator::Neg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_var_reference() {
        let parts = vec!["var".to_string(), "bucket_name".to_string()];
        let expr = parse_dotted_reference(&parts);

        match expr {
            IrExpression::VarRef(name) => assert_eq!(name, "bucket_name"),
            _ => panic!("Expected VarRef"),
        }
    }

    #[test]
    fn test_parse_resource_reference() {
        let parts = vec![
            "aws_s3_bucket".to_string(),
            "example".to_string(),
            "arn".to_string(),
        ];
        let expr = parse_dotted_reference(&parts);

        match expr {
            IrExpression::ResourceRef {
                resource_type,
                name,
                attribute,
            } => {
                assert_eq!(resource_type, "aws_s3_bucket");
                assert_eq!(name, "example");
                assert_eq!(attribute, Some("arn".to_string()));
            }
            _ => panic!("Expected ResourceRef"),
        }
    }

    #[test]
    fn test_parse_template_string() {
        let template = "Hello ${var.name}!";
        let expr = parse_template_string(template);

        match expr {
            IrExpression::Template(parts) => {
                assert_eq!(parts.len(), 3);
                assert!(matches!(&parts[0], TemplatePart::Literal(s) if s == "Hello "));
                assert!(matches!(&parts[1], TemplatePart::Interpolation(_)));
                assert!(matches!(&parts[2], TemplatePart::Literal(s) if s == "!"));
            }
            _ => panic!("Expected Template"),
        }
    }
}
