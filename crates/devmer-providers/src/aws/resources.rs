//! AWS Resource operations
//!
//! This module contains helpers for AWS resource operations.
//! In a production implementation, this would use the AWS SDK.

use devmer_core::resource::Resource;
use devmer_core::types::{PropertyValue, PropertyValues};
use std::collections::HashMap;
use uuid::Uuid;

/// Generate mock outputs for an AWS resource
pub fn generate_outputs(resource: &Resource) -> PropertyValues {
    let resource_type = resource.resource_type.as_str();
    let mut outputs = PropertyValues::new();

    // Generate ARN
    let account_id = "123456789012";
    let region = "us-east-1";

    match resource_type {
        "aws:s3:Bucket" => {
            let bucket_name = resource
                .inputs
                .get("bucket")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!("arn:aws:s3:::{}", bucket_name)),
            );
            outputs.insert(
                "bucketDomainName".to_string(),
                PropertyValue::String(format!("{}.s3.amazonaws.com", bucket_name)),
            );
            outputs.insert(
                "bucketRegionalDomainName".to_string(),
                PropertyValue::String(format!("{}.s3.{}.amazonaws.com", bucket_name, region)),
            );
            outputs.insert(
                "hostedZoneId".to_string(),
                PropertyValue::String("Z3AQBSTGFYJSTF".to_string()), // us-east-1
            );
            outputs.insert("region".to_string(), PropertyValue::String(region.to_string()));

            // Website endpoint if configured
            if resource.inputs.contains_key("website") {
                outputs.insert(
                    "websiteEndpoint".to_string(),
                    PropertyValue::String(format!(
                        "{}.s3-website-{}.amazonaws.com",
                        bucket_name, region
                    )),
                );
            }
        }

        "aws:lambda:Function" => {
            let function_name = resource
                .inputs
                .get("functionName")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:lambda:{}:{}:function:{}",
                    region, account_id, function_name
                )),
            );
            outputs.insert(
                "invokeArn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:apigateway:{}:lambda:path/2015-03-31/functions/arn:aws:lambda:{}:{}:function:{}/invocations",
                    region, region, account_id, function_name
                )),
            );
            outputs.insert(
                "qualifiedArn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:lambda:{}:{}:function:{}:$LATEST",
                    region, account_id, function_name
                )),
            );
            outputs.insert("version".to_string(), PropertyValue::String("$LATEST".to_string()));
            outputs.insert(
                "lastModified".to_string(),
                PropertyValue::String(chrono::Utc::now().to_rfc3339()),
            );
            outputs.insert(
                "sourceCodeHash".to_string(),
                PropertyValue::String(format!("{:x}", md5::compute(function_name.as_bytes()))),
            );
            outputs.insert(
                "sourceCodeSize".to_string(),
                PropertyValue::Int(1024),
            );
        }

        "aws:iam:Role" => {
            let role_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);
            let path = resource
                .inputs
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("/");

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:iam::{}:role{}{}",
                    account_id,
                    path,
                    role_name
                )),
            );
            outputs.insert(
                "uniqueId".to_string(),
                PropertyValue::String(format!("AROA{}", Uuid::new_v4().simple())[..21].to_string()),
            );
            outputs.insert(
                "createDate".to_string(),
                PropertyValue::String(chrono::Utc::now().to_rfc3339()),
            );
        }

        "aws:iam:Policy" => {
            let policy_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);
            let path = resource
                .inputs
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("/");

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:iam::{}:policy{}{}",
                    account_id,
                    path,
                    policy_name
                )),
            );
            outputs.insert(
                "policyId".to_string(),
                PropertyValue::String(format!("ANPA{}", Uuid::new_v4().simple())[..21].to_string()),
            );
        }

        "aws:dynamodb:Table" => {
            let table_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:dynamodb:{}:{}:table/{}",
                    region, account_id, table_name
                )),
            );

            if resource
                .inputs
                .get("streamEnabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                let stream_label = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                outputs.insert(
                    "streamArn".to_string(),
                    PropertyValue::String(format!(
                        "arn:aws:dynamodb:{}:{}:table/{}/stream/{}",
                        region, account_id, table_name, stream_label
                    )),
                );
                outputs.insert(
                    "streamLabel".to_string(),
                    PropertyValue::String(stream_label),
                );
            }
        }

        "aws:ec2:Instance" => {
            let instance_id = format!("i-{}", &Uuid::new_v4().simple().to_string()[..17]);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:ec2:{}:{}:instance/{}",
                    region, account_id, instance_id
                )),
            );
            outputs.insert(
                "publicIp".to_string(),
                PropertyValue::String(format!(
                    "{}.{}.{}.{}",
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                    rand::random::<u8>()
                )),
            );
            outputs.insert(
                "privateIp".to_string(),
                PropertyValue::String(format!(
                    "10.0.{}.{}",
                    rand::random::<u8>(),
                    rand::random::<u8>()
                )),
            );
            outputs.insert(
                "publicDns".to_string(),
                PropertyValue::String(format!(
                    "ec2-{}-{}-{}-{}.compute-1.amazonaws.com",
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                    rand::random::<u8>(),
                    rand::random::<u8>()
                )),
            );
            outputs.insert(
                "privateDns".to_string(),
                PropertyValue::String(format!(
                    "ip-10-0-{}-{}.ec2.internal",
                    rand::random::<u8>(),
                    rand::random::<u8>()
                )),
            );
            outputs.insert(
                "instanceState".to_string(),
                PropertyValue::String("running".to_string()),
            );
            outputs.insert(
                "availabilityZone".to_string(),
                PropertyValue::String(format!("{}a", region)),
            );
        }

        "aws:ec2:SecurityGroup" => {
            let sg_id = format!("sg-{}", &Uuid::new_v4().simple().to_string()[..17]);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:ec2:{}:{}:security-group/{}",
                    region, account_id, sg_id
                )),
            );
            outputs.insert(
                "ownerId".to_string(),
                PropertyValue::String(account_id.to_string()),
            );
        }

        "aws:ec2:Vpc" => {
            let vpc_id = format!("vpc-{}", &Uuid::new_v4().simple().to_string()[..17]);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:ec2:{}:{}:vpc/{}",
                    region, account_id, vpc_id
                )),
            );
            outputs.insert(
                "defaultRouteTableId".to_string(),
                PropertyValue::String(format!("rtb-{}", &Uuid::new_v4().simple().to_string()[..17])),
            );
            outputs.insert(
                "defaultSecurityGroupId".to_string(),
                PropertyValue::String(format!("sg-{}", &Uuid::new_v4().simple().to_string()[..17])),
            );
            outputs.insert(
                "defaultNetworkAclId".to_string(),
                PropertyValue::String(format!("acl-{}", &Uuid::new_v4().simple().to_string()[..17])),
            );
            outputs.insert(
                "ownerId".to_string(),
                PropertyValue::String(account_id.to_string()),
            );
            outputs.insert(
                "mainRouteTableId".to_string(),
                PropertyValue::String(format!("rtb-{}", &Uuid::new_v4().simple().to_string()[..17])),
            );
        }

        "aws:ec2:Subnet" => {
            let subnet_id = format!("subnet-{}", &Uuid::new_v4().simple().to_string()[..17]);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:ec2:{}:{}:subnet/{}",
                    region, account_id, subnet_id
                )),
            );
            outputs.insert(
                "ownerId".to_string(),
                PropertyValue::String(account_id.to_string()),
            );
            outputs.insert(
                "availableIpAddressCount".to_string(),
                PropertyValue::Int(251), // /24 subnet
            );
        }

        "aws:apigatewayv2:Api" => {
            let api_id = Uuid::new_v4().simple().to_string()[..10].to_string();
            let _protocol = resource
                .inputs
                .get("protocolType")
                .and_then(|v| v.as_str())
                .unwrap_or("HTTP");

            outputs.insert(
                "apiEndpoint".to_string(),
                PropertyValue::String(format!("https://{}.execute-api.{}.amazonaws.com", api_id, region)),
            );
            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:apigateway:{}::/apis/{}",
                    region, api_id
                )),
            );
            outputs.insert(
                "executionArn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:execute-api:{}:{}:{}",
                    region, account_id, api_id
                )),
            );
        }

        "aws:sqs:Queue" => {
            let queue_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            let is_fifo = resource
                .inputs
                .get("fifoQueue")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let full_name = if is_fifo && !queue_name.ends_with(".fifo") {
                format!("{}.fifo", queue_name)
            } else {
                queue_name.to_string()
            };

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:sqs:{}:{}:{}",
                    region, account_id, full_name
                )),
            );
            outputs.insert(
                "url".to_string(),
                PropertyValue::String(format!(
                    "https://sqs.{}.amazonaws.com/{}/{}",
                    region, account_id, full_name
                )),
            );
        }

        "aws:sns:Topic" => {
            let topic_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:sns:{}:{}:{}",
                    region, account_id, topic_name
                )),
            );
            outputs.insert(
                "ownerId".to_string(),
                PropertyValue::String(account_id.to_string()),
            );
        }

        "aws:cloudwatch:LogGroup" => {
            let log_group_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:logs:{}:{}:log-group:{}",
                    region, account_id, log_group_name
                )),
            );
        }

        "aws:rds:Instance" => {
            let db_identifier = resource
                .inputs
                .get("identifier")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            let resource_id = format!("db-{}", Uuid::new_v4().simple());

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:rds:{}:{}:db:{}",
                    region, account_id, db_identifier
                )),
            );
            outputs.insert(
                "endpoint".to_string(),
                PropertyValue::String(format!(
                    "{}.{}.{}.rds.amazonaws.com:5432",
                    db_identifier,
                    &resource_id[..8],
                    region
                )),
            );
            outputs.insert(
                "address".to_string(),
                PropertyValue::String(format!(
                    "{}.{}.{}.rds.amazonaws.com",
                    db_identifier,
                    &resource_id[..8],
                    region
                )),
            );
            outputs.insert(
                "hostedZoneId".to_string(),
                PropertyValue::String("Z2R2ITUGPM61AM".to_string()),
            );
            outputs.insert(
                "resourceId".to_string(),
                PropertyValue::String(resource_id),
            );
            outputs.insert("status".to_string(), PropertyValue::String("available".to_string()));
            outputs.insert(
                "availabilityZone".to_string(),
                PropertyValue::String(format!("{}a", region)),
            );
        }

        "aws:secretsmanager:Secret" => {
            let secret_name = resource
                .inputs
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name);

            let secret_id = Uuid::new_v4().simple().to_string()[..6].to_string();

            outputs.insert(
                "arn".to_string(),
                PropertyValue::String(format!(
                    "arn:aws:secretsmanager:{}:{}:secret:{}-{}",
                    region, account_id, secret_name, secret_id
                )),
            );
            outputs.insert(
                "versionId".to_string(),
                PropertyValue::String(Uuid::new_v4().to_string()),
            );
        }

        // Default: generate a generic ARN
        _ => {
            let parts: Vec<&str> = resource_type.split(':').collect();
            if parts.len() >= 3 {
                let service = parts[1];
                let resource_kind = parts[2].to_lowercase();

                outputs.insert(
                    "arn".to_string(),
                    PropertyValue::String(format!(
                        "arn:aws:{}:{}:{}:{}/{}",
                        service, region, account_id, resource_kind, resource.name
                    )),
                );
            }
        }
    }

    // Add the resource ID
    if !outputs.contains_key("id") {
        outputs.insert(
            "id".to_string(),
            PropertyValue::String(resource.id.to_string()),
        );
    }

    outputs
}

/// Validate inputs for an AWS resource
pub fn validate_inputs(
    resource_type: &str,
    inputs: &PropertyValues,
) -> Vec<(String, String)> {
    let mut failures = Vec::new();

    match resource_type {
        "aws:s3:Bucket" => {
            if let Some(bucket) = inputs.get("bucket") {
                if let Some(name) = bucket.as_str() {
                    if name.len() < 3 || name.len() > 63 {
                        failures.push((
                            "bucket".to_string(),
                            "Bucket name must be between 3 and 63 characters".to_string(),
                        ));
                    }
                    if name.starts_with('-') || name.ends_with('-') {
                        failures.push((
                            "bucket".to_string(),
                            "Bucket name cannot start or end with a hyphen".to_string(),
                        ));
                    }
                    if name.contains("..") {
                        failures.push((
                            "bucket".to_string(),
                            "Bucket name cannot contain consecutive periods".to_string(),
                        ));
                    }
                }
            }
        }

        "aws:lambda:Function" => {
            if let Some(memory) = inputs.get("memorySize") {
                if let Some(mem) = memory.as_int() {
                    if mem < 128 || mem > 10240 {
                        failures.push((
                            "memorySize".to_string(),
                            "Memory must be between 128 and 10240 MB".to_string(),
                        ));
                    }
                }
            }
            if let Some(timeout) = inputs.get("timeout") {
                if let Some(t) = timeout.as_int() {
                    if t < 1 || t > 900 {
                        failures.push((
                            "timeout".to_string(),
                            "Timeout must be between 1 and 900 seconds".to_string(),
                        ));
                    }
                }
            }
        }

        "aws:ec2:SecurityGroup" => {
            if let Some(ingress) = inputs.get("ingress") {
                if let PropertyValue::Array(rules) = ingress {
                    for (i, rule) in rules.iter().enumerate() {
                        if let PropertyValue::Object(r) = rule {
                            if let Some(from_port) = r.get("fromPort") {
                                if let Some(port) = from_port.as_int() {
                                    if port < -1 || port > 65535 {
                                        failures.push((
                                            format!("ingress[{}].fromPort", i),
                                            "Port must be between -1 and 65535".to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "aws:ec2:Vpc" => {
            if let Some(cidr) = inputs.get("cidrBlock") {
                if let Some(cidr_str) = cidr.as_str() {
                    // Basic CIDR validation
                    if !cidr_str.contains('/') {
                        failures.push((
                            "cidrBlock".to_string(),
                            "CIDR block must include a prefix length (e.g., /16)".to_string(),
                        ));
                    }
                }
            }
        }

        "aws:sqs:Queue" => {
            if let Some(name) = inputs.get("name") {
                if let Some(queue_name) = name.as_str() {
                    if queue_name.len() > 80 {
                        failures.push((
                            "name".to_string(),
                            "Queue name cannot exceed 80 characters".to_string(),
                        ));
                    }
                }
            }
            if let Some(visibility) = inputs.get("visibilityTimeoutSeconds") {
                if let Some(v) = visibility.as_int() {
                    if v < 0 || v > 43200 {
                        failures.push((
                            "visibilityTimeoutSeconds".to_string(),
                            "Visibility timeout must be between 0 and 43200 seconds".to_string(),
                        ));
                    }
                }
            }
        }

        _ => {}
    }

    failures
}

/// Compute diff between old and new inputs
pub fn compute_diff(
    resource_type: &str,
    old_inputs: &PropertyValues,
    new_inputs: &PropertyValues,
) -> (HashMap<String, String>, bool, Vec<String>) {
    let mut changes = HashMap::new();
    let mut replace = false;
    let mut replace_keys = Vec::new();

    // Properties that force replacement for various resource types
    let force_replace_properties: HashMap<&str, Vec<&str>> = [
        ("aws:s3:Bucket", vec!["bucket"]),
        ("aws:lambda:Function", vec!["functionName"]),
        ("aws:iam:Role", vec!["name", "path"]),
        ("aws:iam:Policy", vec!["name", "path"]),
        ("aws:dynamodb:Table", vec!["name", "hashKey", "rangeKey"]),
        ("aws:ec2:Instance", vec!["ami", "subnetId", "userData"]),
        ("aws:ec2:SecurityGroup", vec!["name", "description", "vpcId"]),
        ("aws:ec2:Vpc", vec!["cidrBlock"]),
        ("aws:ec2:Subnet", vec!["vpcId", "cidrBlock", "availabilityZone"]),
        ("aws:rds:Instance", vec!["identifier", "engine", "username", "dbName"]),
        ("aws:sqs:Queue", vec!["name"]),
        ("aws:sns:Topic", vec!["name"]),
        ("aws:cloudwatch:LogGroup", vec!["name"]),
    ]
    .into_iter()
    .collect();

    let replace_props = force_replace_properties.get(resource_type).cloned().unwrap_or_default();

    // Find all keys
    let mut all_keys: std::collections::HashSet<&String> = old_inputs.keys().collect();
    all_keys.extend(new_inputs.keys());

    for key in all_keys {
        let old_val = old_inputs.get(key);
        let new_val = new_inputs.get(key);

        match (old_val, new_val) {
            (None, Some(_)) => {
                changes.insert(key.clone(), "add".to_string());
                if replace_props.contains(&key.as_str()) {
                    replace = true;
                    replace_keys.push(key.clone());
                }
            }
            (Some(_), None) => {
                changes.insert(key.clone(), "delete".to_string());
                if replace_props.contains(&key.as_str()) {
                    replace = true;
                    replace_keys.push(key.clone());
                }
            }
            (Some(old), Some(new)) if old != new => {
                changes.insert(key.clone(), "update".to_string());
                if replace_props.contains(&key.as_str()) {
                    replace = true;
                    replace_keys.push(key.clone());
                }
            }
            _ => {}
        }
    }

    (changes, replace, replace_keys)
}
