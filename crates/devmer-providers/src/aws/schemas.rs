//! AWS Resource schemas

use devmer_core::provider::{PropertySchema, PropertyType, ResourceSchema};
use devmer_core::resource::ResourceType;
use std::collections::HashMap;

/// Create all AWS resource schemas
pub fn all_schemas() -> HashMap<String, ResourceSchema> {
    let mut schemas = HashMap::new();

    // S3
    schemas.insert("aws:s3:Bucket".to_string(), s3_bucket_schema());
    schemas.insert("aws:s3:BucketPolicy".to_string(), s3_bucket_policy_schema());
    schemas.insert(
        "aws:s3:BucketNotification".to_string(),
        s3_bucket_notification_schema(),
    );

    // Lambda
    schemas.insert("aws:lambda:Function".to_string(), lambda_function_schema());
    schemas.insert("aws:lambda:Permission".to_string(), lambda_permission_schema());
    schemas.insert("aws:lambda:LayerVersion".to_string(), lambda_layer_schema());

    // IAM
    schemas.insert("aws:iam:Role".to_string(), iam_role_schema());
    schemas.insert("aws:iam:Policy".to_string(), iam_policy_schema());
    schemas.insert(
        "aws:iam:RolePolicyAttachment".to_string(),
        iam_role_policy_attachment_schema(),
    );
    schemas.insert("aws:iam:User".to_string(), iam_user_schema());
    schemas.insert(
        "aws:iam:InstanceProfile".to_string(),
        iam_instance_profile_schema(),
    );

    // DynamoDB
    schemas.insert("aws:dynamodb:Table".to_string(), dynamodb_table_schema());

    // EC2
    schemas.insert("aws:ec2:Instance".to_string(), ec2_instance_schema());
    schemas.insert("aws:ec2:SecurityGroup".to_string(), ec2_security_group_schema());
    schemas.insert("aws:ec2:Vpc".to_string(), ec2_vpc_schema());
    schemas.insert("aws:ec2:Subnet".to_string(), ec2_subnet_schema());
    schemas.insert("aws:ec2:InternetGateway".to_string(), ec2_igw_schema());
    schemas.insert("aws:ec2:RouteTable".to_string(), ec2_route_table_schema());
    schemas.insert("aws:ec2:Eip".to_string(), ec2_eip_schema());
    schemas.insert("aws:ec2:NatGateway".to_string(), ec2_nat_gateway_schema());

    // API Gateway v2
    schemas.insert("aws:apigatewayv2:Api".to_string(), apigw_api_schema());
    schemas.insert("aws:apigatewayv2:Stage".to_string(), apigw_stage_schema());
    schemas.insert(
        "aws:apigatewayv2:Integration".to_string(),
        apigw_integration_schema(),
    );
    schemas.insert("aws:apigatewayv2:Route".to_string(), apigw_route_schema());

    // SQS
    schemas.insert("aws:sqs:Queue".to_string(), sqs_queue_schema());

    // SNS
    schemas.insert("aws:sns:Topic".to_string(), sns_topic_schema());
    schemas.insert("aws:sns:Subscription".to_string(), sns_subscription_schema());

    // CloudWatch
    schemas.insert("aws:cloudwatch:LogGroup".to_string(), cloudwatch_log_group_schema());
    schemas.insert(
        "aws:cloudwatch:MetricAlarm".to_string(),
        cloudwatch_metric_alarm_schema(),
    );

    // RDS
    schemas.insert("aws:rds:Instance".to_string(), rds_instance_schema());
    schemas.insert("aws:rds:Cluster".to_string(), rds_cluster_schema());
    schemas.insert("aws:rds:SubnetGroup".to_string(), rds_subnet_group_schema());

    // Secrets Manager
    schemas.insert(
        "aws:secretsmanager:Secret".to_string(),
        secrets_manager_secret_schema(),
    );

    schemas
}

// --- Helper functions ---

fn string_prop(description: &str) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::String,
        description: Some(description.to_string()),
        default: None,
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn string_prop_required(description: &str) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::String,
        description: Some(description.to_string()),
        default: None,
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn string_prop_replace(description: &str) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::String,
        description: Some(description.to_string()),
        default: None,
        secret: false,
        replace_on_change: true,
        deprecated: None,
    }
}

fn bool_prop(description: &str, default: bool) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::Boolean,
        description: Some(description.to_string()),
        default: Some(serde_json::json!(default)),
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn int_prop(description: &str) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::Integer,
        description: Some(description.to_string()),
        default: None,
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn int_prop_with_default(description: &str, default: i64) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::Integer,
        description: Some(description.to_string()),
        default: Some(serde_json::json!(default)),
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn tags_prop() -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::Object(HashMap::new()),
        description: Some("Tags to apply to the resource".to_string()),
        default: None,
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn string_array_prop(description: &str) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::Array(Box::new(PropertyType::String)),
        description: Some(description.to_string()),
        default: None,
        secret: false,
        replace_on_change: false,
        deprecated: None,
    }
}

fn secret_prop(description: &str) -> PropertySchema {
    PropertySchema {
        property_type: PropertyType::String,
        description: Some(description.to_string()),
        default: None,
        secret: true,
        replace_on_change: false,
        deprecated: None,
    }
}

// --- S3 Schemas ---

fn s3_bucket_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("bucket".to_string(), string_prop_replace("The name of the bucket"));
    inputs.insert(
        "acl".to_string(),
        string_prop("The canned ACL to apply (private, public-read, etc.)"),
    );
    inputs.insert("tags".to_string(), tags_prop());
    inputs.insert(
        "versioning".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("enabled".to_string(), bool_prop("Enable versioning", false));
                m
            }),
            description: Some("Versioning configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "website".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("indexDocument".to_string(), string_prop("Index document"));
                m.insert("errorDocument".to_string(), string_prop("Error document"));
                m
            }),
            description: Some("Website hosting configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "forceDestroy".to_string(),
        bool_prop("Delete all objects when destroying bucket", false),
    );

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The ARN of the bucket"));
    outputs.insert(
        "bucketDomainName".to_string(),
        string_prop("The bucket domain name"),
    );
    outputs.insert(
        "bucketRegionalDomainName".to_string(),
        string_prop("The bucket regional domain name"),
    );
    outputs.insert(
        "hostedZoneId".to_string(),
        string_prop("The Route 53 hosted zone ID for the bucket"),
    );
    outputs.insert(
        "region".to_string(),
        string_prop("The AWS region of the bucket"),
    );
    outputs.insert(
        "websiteEndpoint".to_string(),
        string_prop("The website endpoint if configured"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "s3", "Bucket"),
        description: Some("An AWS S3 bucket".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["bucket".to_string()],
    }
}

fn s3_bucket_policy_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "bucket".to_string(),
        string_prop_required("The name of the bucket"),
    );
    inputs.insert(
        "policy".to_string(),
        string_prop_required("The policy document (JSON)"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "s3", "BucketPolicy"),
        description: Some("An S3 bucket policy".to_string()),
        input_properties: inputs,
        output_properties: HashMap::new(),
        required: vec!["bucket".to_string(), "policy".to_string()],
    }
}

fn s3_bucket_notification_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("bucket".to_string(), string_prop_required("The bucket name"));
    inputs.insert(
        "lambdaFunction".to_string(),
        PropertySchema {
            property_type: PropertyType::Array(Box::new(PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("lambdaFunctionArn".to_string(), string_prop("Lambda function ARN"));
                m.insert(
                    "events".to_string(),
                    string_array_prop("Events to trigger on"),
                );
                m.insert("filterPrefix".to_string(), string_prop("Filter prefix"));
                m.insert("filterSuffix".to_string(), string_prop("Filter suffix"));
                m
            }))),
            description: Some("Lambda function notifications".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "s3", "BucketNotification"),
        description: Some("S3 bucket notifications".to_string()),
        input_properties: inputs,
        output_properties: HashMap::new(),
        required: vec!["bucket".to_string()],
    }
}

// --- Lambda Schemas ---

fn lambda_function_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "functionName".to_string(),
        string_prop_replace("The function name"),
    );
    inputs.insert("runtime".to_string(), string_prop("The runtime (e.g., nodejs18.x, python3.11)"));
    inputs.insert("handler".to_string(), string_prop("The function handler"));
    inputs.insert("role".to_string(), string_prop_required("The IAM role ARN"));
    inputs.insert(
        "code".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("s3Bucket".to_string(), string_prop("S3 bucket containing the code"));
                m.insert("s3Key".to_string(), string_prop("S3 key for the code"));
                m.insert(
                    "s3ObjectVersion".to_string(),
                    string_prop("S3 object version"),
                );
                m.insert("zipFile".to_string(), string_prop("Base64-encoded zip file"));
                m.insert("imageUri".to_string(), string_prop("Container image URI"));
                m
            }),
            description: Some("The function code".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "memorySize".to_string(),
        int_prop_with_default("Memory size in MB", 128),
    );
    inputs.insert(
        "timeout".to_string(),
        int_prop_with_default("Timeout in seconds", 3),
    );
    inputs.insert(
        "environment".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert(
                    "variables".to_string(),
                    PropertySchema {
                        property_type: PropertyType::Object(HashMap::new()),
                        description: Some("Environment variables".to_string()),
                        default: None,
                        secret: false,
                        replace_on_change: false,
                        deprecated: None,
                    },
                );
                m
            }),
            description: Some("Environment configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "vpcConfig".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("subnetIds".to_string(), string_array_prop("Subnet IDs"));
                m.insert(
                    "securityGroupIds".to_string(),
                    string_array_prop("Security group IDs"),
                );
                m
            }),
            description: Some("VPC configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "layers".to_string(),
        string_array_prop("Lambda layer ARNs"),
    );
    inputs.insert("tags".to_string(), tags_prop());
    inputs.insert(
        "architectures".to_string(),
        string_array_prop("Instruction set architectures (x86_64, arm64)"),
    );
    inputs.insert(
        "reservedConcurrentExecutions".to_string(),
        int_prop("Reserved concurrent executions"),
    );
    inputs.insert(
        "publish".to_string(),
        bool_prop("Publish a new version", false),
    );

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The function ARN"));
    outputs.insert(
        "invokeArn".to_string(),
        string_prop("The ARN to invoke the function"),
    );
    outputs.insert(
        "qualifiedArn".to_string(),
        string_prop("The qualified ARN (with version)"),
    );
    outputs.insert("version".to_string(), string_prop("The function version"));
    outputs.insert(
        "lastModified".to_string(),
        string_prop("Last modified timestamp"),
    );
    outputs.insert(
        "sourceCodeHash".to_string(),
        string_prop("Hash of the source code"),
    );
    outputs.insert(
        "sourceCodeSize".to_string(),
        int_prop("Size of the source code in bytes"),
    );
    outputs.insert(
        "signingJobArn".to_string(),
        string_prop("ARN of the signing job"),
    );
    outputs.insert(
        "signingProfileVersionArn".to_string(),
        string_prop("ARN of the signing profile version"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "lambda", "Function"),
        description: Some("An AWS Lambda function".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["functionName".to_string(), "role".to_string()],
    }
}

fn lambda_permission_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "functionName".to_string(),
        string_prop_required("The function name"),
    );
    inputs.insert("action".to_string(), string_prop_required("The action"));
    inputs.insert(
        "principal".to_string(),
        string_prop_required("The principal (e.g., s3.amazonaws.com)"),
    );
    inputs.insert(
        "sourceArn".to_string(),
        string_prop("The source ARN for the permission"),
    );
    inputs.insert(
        "sourceAccount".to_string(),
        string_prop("The source account ID"),
    );
    inputs.insert(
        "statementId".to_string(),
        string_prop_replace("Statement ID"),
    );
    inputs.insert("qualifier".to_string(), string_prop("Lambda qualifier"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "lambda", "Permission"),
        description: Some("A Lambda function permission".to_string()),
        input_properties: inputs,
        output_properties: HashMap::new(),
        required: vec![
            "functionName".to_string(),
            "action".to_string(),
            "principal".to_string(),
        ],
    }
}

fn lambda_layer_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "layerName".to_string(),
        string_prop_replace("The layer name"),
    );
    inputs.insert(
        "compatibleRuntimes".to_string(),
        string_array_prop("Compatible runtimes"),
    );
    inputs.insert(
        "compatibleArchitectures".to_string(),
        string_array_prop("Compatible architectures"),
    );
    inputs.insert(
        "code".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("s3Bucket".to_string(), string_prop("S3 bucket"));
                m.insert("s3Key".to_string(), string_prop("S3 key"));
                m
            }),
            description: Some("The layer code".to_string()),
            default: None,
            secret: false,
            replace_on_change: true,
            deprecated: None,
        },
    );
    inputs.insert("description".to_string(), string_prop("Layer description"));

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The layer ARN"));
    outputs.insert(
        "layerArn".to_string(),
        string_prop("The layer ARN without version"),
    );
    outputs.insert("version".to_string(), int_prop("The layer version"));
    outputs.insert(
        "createdDate".to_string(),
        string_prop("Creation timestamp"),
    );
    outputs.insert(
        "sourceCodeSize".to_string(),
        int_prop("Size of the layer in bytes"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "lambda", "LayerVersion"),
        description: Some("A Lambda layer version".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["layerName".to_string()],
    }
}

// --- IAM Schemas ---

fn iam_role_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The role name"));
    inputs.insert(
        "assumeRolePolicy".to_string(),
        string_prop_required("The assume role policy document (JSON)"),
    );
    inputs.insert(
        "description".to_string(),
        string_prop("Description of the role"),
    );
    inputs.insert("path".to_string(), string_prop_replace("The path for the role"));
    inputs.insert(
        "maxSessionDuration".to_string(),
        int_prop_with_default("Maximum session duration in seconds", 3600),
    );
    inputs.insert(
        "permissionsBoundary".to_string(),
        string_prop("ARN of the permissions boundary policy"),
    );
    inputs.insert(
        "forceDetachPolicies".to_string(),
        bool_prop("Force detach policies on destroy", false),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The role ARN"));
    outputs.insert("uniqueId".to_string(), string_prop("The unique ID"));
    outputs.insert(
        "createDate".to_string(),
        string_prop("Creation timestamp"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "iam", "Role"),
        description: Some("An IAM role".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string(), "assumeRolePolicy".to_string()],
    }
}

fn iam_policy_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The policy name"));
    inputs.insert(
        "policy".to_string(),
        string_prop_required("The policy document (JSON)"),
    );
    inputs.insert(
        "description".to_string(),
        string_prop("Description of the policy"),
    );
    inputs.insert("path".to_string(), string_prop_replace("The path for the policy"));
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The policy ARN"));
    outputs.insert(
        "policyId".to_string(),
        string_prop("The policy ID"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "iam", "Policy"),
        description: Some("An IAM policy".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string(), "policy".to_string()],
    }
}

fn iam_role_policy_attachment_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "role".to_string(),
        string_prop_required("The role name"),
    );
    inputs.insert(
        "policyArn".to_string(),
        string_prop_required("The policy ARN to attach"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "iam", "RolePolicyAttachment"),
        description: Some("Attaches a policy to an IAM role".to_string()),
        input_properties: inputs,
        output_properties: HashMap::new(),
        required: vec!["role".to_string(), "policyArn".to_string()],
    }
}

fn iam_user_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The user name"));
    inputs.insert("path".to_string(), string_prop_replace("The path"));
    inputs.insert(
        "permissionsBoundary".to_string(),
        string_prop("Permissions boundary ARN"),
    );
    inputs.insert(
        "forceDestroy".to_string(),
        bool_prop("Force destroy access keys", false),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The user ARN"));
    outputs.insert("uniqueId".to_string(), string_prop("The unique ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "iam", "User"),
        description: Some("An IAM user".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string()],
    }
}

fn iam_instance_profile_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The profile name"));
    inputs.insert("role".to_string(), string_prop("The role name to attach"));
    inputs.insert("path".to_string(), string_prop_replace("The path"));
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The profile ARN"));
    outputs.insert("uniqueId".to_string(), string_prop("The unique ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "iam", "InstanceProfile"),
        description: Some("An IAM instance profile".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string()],
    }
}

// --- DynamoDB Schema ---

fn dynamodb_table_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The table name"));
    inputs.insert(
        "billingMode".to_string(),
        string_prop("Billing mode (PROVISIONED or PAY_PER_REQUEST)"),
    );
    inputs.insert(
        "hashKey".to_string(),
        string_prop_replace("The hash key attribute name"),
    );
    inputs.insert(
        "rangeKey".to_string(),
        string_prop_replace("The range key attribute name"),
    );
    inputs.insert(
        "readCapacity".to_string(),
        int_prop("Read capacity units (for PROVISIONED)"),
    );
    inputs.insert(
        "writeCapacity".to_string(),
        int_prop("Write capacity units (for PROVISIONED)"),
    );
    inputs.insert(
        "attributes".to_string(),
        PropertySchema {
            property_type: PropertyType::Array(Box::new(PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("name".to_string(), string_prop("Attribute name"));
                m.insert(
                    "type".to_string(),
                    string_prop("Attribute type (S, N, B)"),
                );
                m
            }))),
            description: Some("Attribute definitions".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "globalSecondaryIndexes".to_string(),
        PropertySchema {
            property_type: PropertyType::Array(Box::new(PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("name".to_string(), string_prop("Index name"));
                m.insert("hashKey".to_string(), string_prop("Hash key"));
                m.insert("rangeKey".to_string(), string_prop("Range key"));
                m.insert(
                    "projectionType".to_string(),
                    string_prop("Projection type"),
                );
                m
            }))),
            description: Some("Global secondary indexes".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "ttl".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("enabled".to_string(), bool_prop("Enable TTL", false));
                m.insert(
                    "attributeName".to_string(),
                    string_prop("TTL attribute name"),
                );
                m
            }),
            description: Some("TTL configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "streamEnabled".to_string(),
        bool_prop("Enable DynamoDB Streams", false),
    );
    inputs.insert(
        "streamViewType".to_string(),
        string_prop("Stream view type"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The table ARN"));
    outputs.insert("streamArn".to_string(), string_prop("The stream ARN"));
    outputs.insert(
        "streamLabel".to_string(),
        string_prop("The stream label"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "dynamodb", "Table"),
        description: Some("A DynamoDB table".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![
            "name".to_string(),
            "hashKey".to_string(),
            "attributes".to_string(),
        ],
    }
}

// --- EC2 Schemas ---

fn ec2_instance_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("ami".to_string(), string_prop_replace("The AMI ID"));
    inputs.insert(
        "instanceType".to_string(),
        string_prop("The instance type (e.g., t3.micro)"),
    );
    inputs.insert("keyName".to_string(), string_prop("The key pair name"));
    inputs.insert(
        "subnetId".to_string(),
        string_prop_replace("The subnet ID"),
    );
    inputs.insert(
        "vpcSecurityGroupIds".to_string(),
        string_array_prop("VPC security group IDs"),
    );
    inputs.insert(
        "iamInstanceProfile".to_string(),
        string_prop("IAM instance profile"),
    );
    inputs.insert("userData".to_string(), string_prop_replace("User data script"));
    inputs.insert(
        "associatePublicIpAddress".to_string(),
        bool_prop("Associate public IP", false),
    );
    inputs.insert(
        "rootBlockDevice".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert(
                    "volumeSize".to_string(),
                    int_prop("Volume size in GB"),
                );
                m.insert("volumeType".to_string(), string_prop("Volume type"));
                m.insert("encrypted".to_string(), bool_prop("Encrypt volume", false));
                m.insert(
                    "deleteOnTermination".to_string(),
                    bool_prop("Delete on termination", true),
                );
                m
            }),
            description: Some("Root block device configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "monitoring".to_string(),
        bool_prop("Enable detailed monitoring", false),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The instance ARN"));
    outputs.insert(
        "publicIp".to_string(),
        string_prop("The public IP address"),
    );
    outputs.insert(
        "privateIp".to_string(),
        string_prop("The private IP address"),
    );
    outputs.insert(
        "publicDns".to_string(),
        string_prop("The public DNS name"),
    );
    outputs.insert(
        "privateDns".to_string(),
        string_prop("The private DNS name"),
    );
    outputs.insert(
        "instanceState".to_string(),
        string_prop("The instance state"),
    );
    outputs.insert(
        "availabilityZone".to_string(),
        string_prop("The availability zone"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "Instance"),
        description: Some("An EC2 instance".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["ami".to_string(), "instanceType".to_string()],
    }
}

fn ec2_security_group_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The security group name"));
    inputs.insert(
        "description".to_string(),
        string_prop_replace("The security group description"),
    );
    inputs.insert("vpcId".to_string(), string_prop_replace("The VPC ID"));
    inputs.insert(
        "ingress".to_string(),
        PropertySchema {
            property_type: PropertyType::Array(Box::new(PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("fromPort".to_string(), int_prop("From port"));
                m.insert("toPort".to_string(), int_prop("To port"));
                m.insert("protocol".to_string(), string_prop("Protocol"));
                m.insert("cidrBlocks".to_string(), string_array_prop("CIDR blocks"));
                m.insert(
                    "securityGroups".to_string(),
                    string_array_prop("Security group IDs"),
                );
                m.insert("self".to_string(), bool_prop("Self reference", false));
                m.insert("description".to_string(), string_prop("Rule description"));
                m
            }))),
            description: Some("Ingress rules".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "egress".to_string(),
        PropertySchema {
            property_type: PropertyType::Array(Box::new(PropertyType::Object({
                let mut m = HashMap::new();
                m.insert("fromPort".to_string(), int_prop("From port"));
                m.insert("toPort".to_string(), int_prop("To port"));
                m.insert("protocol".to_string(), string_prop("Protocol"));
                m.insert("cidrBlocks".to_string(), string_array_prop("CIDR blocks"));
                m.insert(
                    "securityGroups".to_string(),
                    string_array_prop("Security group IDs"),
                );
                m
            }))),
            description: Some("Egress rules".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The security group ARN"));
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "SecurityGroup"),
        description: Some("An EC2 security group".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string(), "description".to_string()],
    }
}

fn ec2_vpc_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "cidrBlock".to_string(),
        string_prop_replace("The CIDR block for the VPC"),
    );
    inputs.insert(
        "enableDnsHostnames".to_string(),
        bool_prop("Enable DNS hostnames", false),
    );
    inputs.insert(
        "enableDnsSupport".to_string(),
        bool_prop("Enable DNS support", true),
    );
    inputs.insert(
        "instanceTenancy".to_string(),
        string_prop("Instance tenancy (default, dedicated)"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The VPC ARN"));
    outputs.insert(
        "defaultRouteTableId".to_string(),
        string_prop("The default route table ID"),
    );
    outputs.insert(
        "defaultSecurityGroupId".to_string(),
        string_prop("The default security group ID"),
    );
    outputs.insert(
        "defaultNetworkAclId".to_string(),
        string_prop("The default network ACL ID"),
    );
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));
    outputs.insert(
        "mainRouteTableId".to_string(),
        string_prop("The main route table ID"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "Vpc"),
        description: Some("A VPC".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["cidrBlock".to_string()],
    }
}

fn ec2_subnet_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("vpcId".to_string(), string_prop_replace("The VPC ID"));
    inputs.insert(
        "cidrBlock".to_string(),
        string_prop_replace("The CIDR block for the subnet"),
    );
    inputs.insert(
        "availabilityZone".to_string(),
        string_prop_replace("The availability zone"),
    );
    inputs.insert(
        "mapPublicIpOnLaunch".to_string(),
        bool_prop("Map public IP on launch", false),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The subnet ARN"));
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));
    outputs.insert(
        "availableIpAddressCount".to_string(),
        int_prop("Available IP address count"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "Subnet"),
        description: Some("A VPC subnet".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["vpcId".to_string(), "cidrBlock".to_string()],
    }
}

fn ec2_igw_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("vpcId".to_string(), string_prop("The VPC ID to attach to"));
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The IGW ARN"));
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "InternetGateway"),
        description: Some("An Internet Gateway".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![],
    }
}

fn ec2_route_table_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("vpcId".to_string(), string_prop_replace("The VPC ID"));
    inputs.insert(
        "routes".to_string(),
        PropertySchema {
            property_type: PropertyType::Array(Box::new(PropertyType::Object({
                let mut m = HashMap::new();
                m.insert(
                    "cidrBlock".to_string(),
                    string_prop("Destination CIDR block"),
                );
                m.insert("gatewayId".to_string(), string_prop("Gateway ID"));
                m.insert("natGatewayId".to_string(), string_prop("NAT Gateway ID"));
                m.insert("instanceId".to_string(), string_prop("Instance ID"));
                m.insert(
                    "networkInterfaceId".to_string(),
                    string_prop("Network interface ID"),
                );
                m
            }))),
            description: Some("Routes".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The route table ARN"));
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "RouteTable"),
        description: Some("A route table".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["vpcId".to_string()],
    }
}

fn ec2_eip_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("vpc".to_string(), bool_prop("Allocate for VPC", true));
    inputs.insert("instance".to_string(), string_prop("EC2 instance ID"));
    inputs.insert(
        "networkInterface".to_string(),
        string_prop("Network interface ID"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert(
        "allocationId".to_string(),
        string_prop("The allocation ID"),
    );
    outputs.insert(
        "associationId".to_string(),
        string_prop("The association ID"),
    );
    outputs.insert("publicIp".to_string(), string_prop("The public IP"));
    outputs.insert("privateDns".to_string(), string_prop("The private DNS"));
    outputs.insert(
        "publicDns".to_string(),
        string_prop("The public DNS name"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "Eip"),
        description: Some("An Elastic IP address".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![],
    }
}

fn ec2_nat_gateway_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "allocationId".to_string(),
        string_prop("EIP allocation ID"),
    );
    inputs.insert("subnetId".to_string(), string_prop_required("Subnet ID"));
    inputs.insert(
        "connectivityType".to_string(),
        string_prop("Connectivity type (public, private)"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert(
        "networkInterfaceId".to_string(),
        string_prop("Network interface ID"),
    );
    outputs.insert("privateIp".to_string(), string_prop("Private IP"));
    outputs.insert("publicIp".to_string(), string_prop("Public IP"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "ec2", "NatGateway"),
        description: Some("A NAT Gateway".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["subnetId".to_string()],
    }
}

// --- API Gateway v2 Schemas ---

fn apigw_api_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_required("The API name"));
    inputs.insert(
        "protocolType".to_string(),
        string_prop_replace("Protocol type (HTTP, WEBSOCKET)"),
    );
    inputs.insert("description".to_string(), string_prop("API description"));
    inputs.insert(
        "corsConfiguration".to_string(),
        PropertySchema {
            property_type: PropertyType::Object({
                let mut m = HashMap::new();
                m.insert(
                    "allowOrigins".to_string(),
                    string_array_prop("Allowed origins"),
                );
                m.insert(
                    "allowMethods".to_string(),
                    string_array_prop("Allowed methods"),
                );
                m.insert(
                    "allowHeaders".to_string(),
                    string_array_prop("Allowed headers"),
                );
                m.insert("maxAge".to_string(), int_prop("Max age in seconds"));
                m.insert(
                    "allowCredentials".to_string(),
                    bool_prop("Allow credentials", false),
                );
                m
            }),
            description: Some("CORS configuration".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert(
        "apiEndpoint".to_string(),
        string_prop("The API endpoint"),
    );
    outputs.insert("arn".to_string(), string_prop("The API ARN"));
    outputs.insert(
        "executionArn".to_string(),
        string_prop("The execution ARN"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "apigatewayv2", "Api"),
        description: Some("An API Gateway v2 API".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string(), "protocolType".to_string()],
    }
}

fn apigw_stage_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("apiId".to_string(), string_prop_required("The API ID"));
    inputs.insert("name".to_string(), string_prop_replace("The stage name"));
    inputs.insert("autoDeploy".to_string(), bool_prop("Auto deploy", false));
    inputs.insert("description".to_string(), string_prop("Stage description"));
    inputs.insert(
        "stageVariables".to_string(),
        PropertySchema {
            property_type: PropertyType::Object(HashMap::new()),
            description: Some("Stage variables".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The stage ARN"));
    outputs.insert("invokeUrl".to_string(), string_prop("The invoke URL"));
    outputs.insert(
        "executionArn".to_string(),
        string_prop("The execution ARN"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "apigatewayv2", "Stage"),
        description: Some("An API Gateway v2 stage".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["apiId".to_string(), "name".to_string()],
    }
}

fn apigw_integration_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("apiId".to_string(), string_prop_required("The API ID"));
    inputs.insert(
        "integrationType".to_string(),
        string_prop_required("Integration type (AWS_PROXY, HTTP_PROXY, etc.)"),
    );
    inputs.insert(
        "integrationUri".to_string(),
        string_prop("Integration URI (Lambda ARN, HTTP URL)"),
    );
    inputs.insert(
        "integrationMethod".to_string(),
        string_prop("Integration HTTP method"),
    );
    inputs.insert(
        "connectionType".to_string(),
        string_prop("Connection type (INTERNET, VPC_LINK)"),
    );
    inputs.insert(
        "payloadFormatVersion".to_string(),
        string_prop("Payload format version"),
    );
    inputs.insert(
        "timeoutMilliseconds".to_string(),
        int_prop_with_default("Timeout in milliseconds", 30000),
    );

    let mut outputs = HashMap::new();
    outputs.insert(
        "integrationResponseSelectionExpression".to_string(),
        string_prop("Response selection expression"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "apigatewayv2", "Integration"),
        description: Some("An API Gateway v2 integration".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["apiId".to_string(), "integrationType".to_string()],
    }
}

fn apigw_route_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("apiId".to_string(), string_prop_required("The API ID"));
    inputs.insert(
        "routeKey".to_string(),
        string_prop_required("Route key (e.g., 'GET /items')"),
    );
    inputs.insert("target".to_string(), string_prop("Integration target"));
    inputs.insert(
        "authorizationType".to_string(),
        string_prop("Authorization type (NONE, JWT, AWS_IAM, CUSTOM)"),
    );
    inputs.insert("authorizerId".to_string(), string_prop("Authorizer ID"));
    inputs.insert(
        "operationName".to_string(),
        string_prop("Operation name"),
    );

    let mut outputs = HashMap::new();
    outputs.insert("routeId".to_string(), string_prop("The route ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "apigatewayv2", "Route"),
        description: Some("An API Gateway v2 route".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["apiId".to_string(), "routeKey".to_string()],
    }
}

// --- SQS Schema ---

fn sqs_queue_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The queue name"));
    inputs.insert(
        "fifoQueue".to_string(),
        bool_prop("Is this a FIFO queue", false),
    );
    inputs.insert(
        "visibilityTimeoutSeconds".to_string(),
        int_prop_with_default("Visibility timeout in seconds", 30),
    );
    inputs.insert(
        "messageRetentionSeconds".to_string(),
        int_prop_with_default("Message retention in seconds", 345600),
    );
    inputs.insert(
        "maxMessageSize".to_string(),
        int_prop_with_default("Max message size in bytes", 262144),
    );
    inputs.insert(
        "delaySeconds".to_string(),
        int_prop_with_default("Delivery delay in seconds", 0),
    );
    inputs.insert(
        "receiveWaitTimeSeconds".to_string(),
        int_prop_with_default("Receive wait time in seconds", 0),
    );
    inputs.insert(
        "redrivePolicy".to_string(),
        string_prop("Redrive policy JSON for DLQ"),
    );
    inputs.insert(
        "contentBasedDeduplication".to_string(),
        bool_prop("Content-based deduplication (FIFO only)", false),
    );
    inputs.insert(
        "sqsManagedSseEnabled".to_string(),
        bool_prop("Enable SQS-managed SSE", false),
    );
    inputs.insert(
        "kmsMasterKeyId".to_string(),
        string_prop("KMS key ID for SSE"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The queue ARN"));
    outputs.insert("url".to_string(), string_prop("The queue URL"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "sqs", "Queue"),
        description: Some("An SQS queue".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string()],
    }
}

// --- SNS Schemas ---

fn sns_topic_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The topic name"));
    inputs.insert("displayName".to_string(), string_prop("Display name"));
    inputs.insert(
        "fifoTopic".to_string(),
        bool_prop("Is this a FIFO topic", false),
    );
    inputs.insert(
        "contentBasedDeduplication".to_string(),
        bool_prop("Content-based deduplication", false),
    );
    inputs.insert(
        "kmsMasterKeyId".to_string(),
        string_prop("KMS key ID for encryption"),
    );
    inputs.insert("policy".to_string(), string_prop("Access policy JSON"));
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The topic ARN"));
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "sns", "Topic"),
        description: Some("An SNS topic".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string()],
    }
}

fn sns_subscription_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("topicArn".to_string(), string_prop_required("The topic ARN"));
    inputs.insert(
        "protocol".to_string(),
        string_prop_required("Protocol (email, sqs, lambda, https, etc.)"),
    );
    inputs.insert(
        "endpoint".to_string(),
        string_prop_required("Endpoint for the subscription"),
    );
    inputs.insert(
        "filterPolicy".to_string(),
        string_prop("Filter policy JSON"),
    );
    inputs.insert(
        "rawMessageDelivery".to_string(),
        bool_prop("Raw message delivery", false),
    );

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The subscription ARN"));
    outputs.insert(
        "confirmationWasAuthenticated".to_string(),
        bool_prop("Was confirmation authenticated", false),
    );
    outputs.insert("ownerId".to_string(), string_prop("The owner ID"));
    outputs.insert(
        "pendingConfirmation".to_string(),
        bool_prop("Pending confirmation", false),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "sns", "Subscription"),
        description: Some("An SNS subscription".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![
            "topicArn".to_string(),
            "protocol".to_string(),
            "endpoint".to_string(),
        ],
    }
}

// --- CloudWatch Schemas ---

fn cloudwatch_log_group_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The log group name"));
    inputs.insert(
        "retentionInDays".to_string(),
        int_prop("Retention in days (0 = never expire)"),
    );
    inputs.insert(
        "kmsKeyId".to_string(),
        string_prop("KMS key ARN for encryption"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The log group ARN"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "cloudwatch", "LogGroup"),
        description: Some("A CloudWatch log group".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string()],
    }
}

fn cloudwatch_metric_alarm_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "alarmName".to_string(),
        string_prop_replace("The alarm name"),
    );
    inputs.insert(
        "comparisonOperator".to_string(),
        string_prop_required("Comparison operator"),
    );
    inputs.insert(
        "evaluationPeriods".to_string(),
        int_prop("Number of periods to evaluate"),
    );
    inputs.insert("metricName".to_string(), string_prop("Metric name"));
    inputs.insert("namespace".to_string(), string_prop("Metric namespace"));
    inputs.insert("period".to_string(), int_prop("Period in seconds"));
    inputs.insert(
        "statistic".to_string(),
        string_prop("Statistic (Average, Sum, etc.)"),
    );
    inputs.insert("threshold".to_string(), int_prop("Threshold value"));
    inputs.insert(
        "alarmDescription".to_string(),
        string_prop("Alarm description"),
    );
    inputs.insert(
        "alarmActions".to_string(),
        string_array_prop("Alarm action ARNs"),
    );
    inputs.insert(
        "okActions".to_string(),
        string_array_prop("OK action ARNs"),
    );
    inputs.insert(
        "insufficientDataActions".to_string(),
        string_array_prop("Insufficient data action ARNs"),
    );
    inputs.insert(
        "dimensions".to_string(),
        PropertySchema {
            property_type: PropertyType::Object(HashMap::new()),
            description: Some("Metric dimensions".to_string()),
            default: None,
            secret: false,
            replace_on_change: false,
            deprecated: None,
        },
    );
    inputs.insert(
        "treatMissingData".to_string(),
        string_prop("How to treat missing data"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The alarm ARN"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "cloudwatch", "MetricAlarm"),
        description: Some("A CloudWatch metric alarm".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![
            "alarmName".to_string(),
            "comparisonOperator".to_string(),
            "evaluationPeriods".to_string(),
        ],
    }
}

// --- RDS Schemas ---

fn rds_instance_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "identifier".to_string(),
        string_prop_replace("The DB instance identifier"),
    );
    inputs.insert(
        "instanceClass".to_string(),
        string_prop("The instance class (e.g., db.t3.micro)"),
    );
    inputs.insert("engine".to_string(), string_prop_replace("The database engine"));
    inputs.insert(
        "engineVersion".to_string(),
        string_prop("The engine version"),
    );
    inputs.insert(
        "allocatedStorage".to_string(),
        int_prop("Allocated storage in GB"),
    );
    inputs.insert(
        "maxAllocatedStorage".to_string(),
        int_prop("Max allocated storage for autoscaling"),
    );
    inputs.insert(
        "storageType".to_string(),
        string_prop("Storage type (gp2, gp3, io1)"),
    );
    inputs.insert(
        "storageEncrypted".to_string(),
        bool_prop("Encrypt storage", false),
    );
    inputs.insert("kmsKeyId".to_string(), string_prop("KMS key ID"));
    inputs.insert(
        "username".to_string(),
        string_prop_replace("Master username"),
    );
    inputs.insert("password".to_string(), secret_prop("Master password"));
    inputs.insert("dbName".to_string(), string_prop_replace("Database name"));
    inputs.insert("port".to_string(), int_prop("Database port"));
    inputs.insert(
        "vpcSecurityGroupIds".to_string(),
        string_array_prop("VPC security group IDs"),
    );
    inputs.insert(
        "dbSubnetGroupName".to_string(),
        string_prop("DB subnet group name"),
    );
    inputs.insert(
        "parameterGroupName".to_string(),
        string_prop("Parameter group name"),
    );
    inputs.insert(
        "optionGroupName".to_string(),
        string_prop("Option group name"),
    );
    inputs.insert(
        "publiclyAccessible".to_string(),
        bool_prop("Publicly accessible", false),
    );
    inputs.insert(
        "multiAz".to_string(),
        bool_prop("Multi-AZ deployment", false),
    );
    inputs.insert(
        "autoMinorVersionUpgrade".to_string(),
        bool_prop("Auto minor version upgrade", true),
    );
    inputs.insert(
        "backupRetentionPeriod".to_string(),
        int_prop_with_default("Backup retention period in days", 7),
    );
    inputs.insert(
        "backupWindow".to_string(),
        string_prop("Preferred backup window"),
    );
    inputs.insert(
        "maintenanceWindow".to_string(),
        string_prop("Preferred maintenance window"),
    );
    inputs.insert(
        "deletionProtection".to_string(),
        bool_prop("Enable deletion protection", false),
    );
    inputs.insert(
        "skipFinalSnapshot".to_string(),
        bool_prop("Skip final snapshot on deletion", false),
    );
    inputs.insert(
        "finalSnapshotIdentifier".to_string(),
        string_prop("Final snapshot identifier"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The DB instance ARN"));
    outputs.insert("endpoint".to_string(), string_prop("The endpoint"));
    outputs.insert("address".to_string(), string_prop("The address"));
    outputs.insert(
        "hostedZoneId".to_string(),
        string_prop("The hosted zone ID"),
    );
    outputs.insert(
        "resourceId".to_string(),
        string_prop("The resource ID"),
    );
    outputs.insert("status".to_string(), string_prop("The status"));
    outputs.insert(
        "availabilityZone".to_string(),
        string_prop("The availability zone"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "rds", "Instance"),
        description: Some("An RDS database instance".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![
            "identifier".to_string(),
            "instanceClass".to_string(),
            "engine".to_string(),
        ],
    }
}

fn rds_cluster_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert(
        "clusterIdentifier".to_string(),
        string_prop_replace("The cluster identifier"),
    );
    inputs.insert("engine".to_string(), string_prop_replace("The database engine"));
    inputs.insert(
        "engineMode".to_string(),
        string_prop("Engine mode (provisioned, serverless)"),
    );
    inputs.insert(
        "engineVersion".to_string(),
        string_prop("The engine version"),
    );
    inputs.insert(
        "masterUsername".to_string(),
        string_prop_replace("Master username"),
    );
    inputs.insert("masterPassword".to_string(), secret_prop("Master password"));
    inputs.insert("databaseName".to_string(), string_prop_replace("Database name"));
    inputs.insert("port".to_string(), int_prop("Database port"));
    inputs.insert(
        "vpcSecurityGroupIds".to_string(),
        string_array_prop("VPC security group IDs"),
    );
    inputs.insert(
        "dbSubnetGroupName".to_string(),
        string_prop("DB subnet group name"),
    );
    inputs.insert(
        "storageEncrypted".to_string(),
        bool_prop("Encrypt storage", false),
    );
    inputs.insert("kmsKeyId".to_string(), string_prop("KMS key ID"));
    inputs.insert(
        "backupRetentionPeriod".to_string(),
        int_prop_with_default("Backup retention period", 7),
    );
    inputs.insert(
        "preferredBackupWindow".to_string(),
        string_prop("Preferred backup window"),
    );
    inputs.insert(
        "preferredMaintenanceWindow".to_string(),
        string_prop("Preferred maintenance window"),
    );
    inputs.insert(
        "skipFinalSnapshot".to_string(),
        bool_prop("Skip final snapshot", false),
    );
    inputs.insert(
        "deletionProtection".to_string(),
        bool_prop("Enable deletion protection", false),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The cluster ARN"));
    outputs.insert("endpoint".to_string(), string_prop("The writer endpoint"));
    outputs.insert(
        "readerEndpoint".to_string(),
        string_prop("The reader endpoint"),
    );
    outputs.insert(
        "hostedZoneId".to_string(),
        string_prop("The hosted zone ID"),
    );
    outputs.insert(
        "clusterResourceId".to_string(),
        string_prop("The cluster resource ID"),
    );

    ResourceSchema {
        resource_type: ResourceType::new("aws", "rds", "Cluster"),
        description: Some("An RDS Aurora cluster".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec![
            "clusterIdentifier".to_string(),
            "engine".to_string(),
        ],
    }
}

fn rds_subnet_group_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The subnet group name"));
    inputs.insert(
        "description".to_string(),
        string_prop("Subnet group description"),
    );
    inputs.insert(
        "subnetIds".to_string(),
        string_array_prop("List of subnet IDs"),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The subnet group ARN"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "rds", "SubnetGroup"),
        description: Some("An RDS DB subnet group".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string(), "subnetIds".to_string()],
    }
}

// --- Secrets Manager Schema ---

fn secrets_manager_secret_schema() -> ResourceSchema {
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), string_prop_replace("The secret name"));
    inputs.insert("description".to_string(), string_prop("Secret description"));
    inputs.insert("kmsKeyId".to_string(), string_prop("KMS key ID"));
    inputs.insert("secretString".to_string(), secret_prop("The secret value"));
    inputs.insert(
        "recoveryWindowInDays".to_string(),
        int_prop_with_default("Recovery window in days (0 to force delete)", 30),
    );
    inputs.insert(
        "forceOverwriteReplicaSecret".to_string(),
        bool_prop("Force overwrite replica secret", false),
    );
    inputs.insert("tags".to_string(), tags_prop());

    let mut outputs = HashMap::new();
    outputs.insert("arn".to_string(), string_prop("The secret ARN"));
    outputs.insert("versionId".to_string(), string_prop("The version ID"));

    ResourceSchema {
        resource_type: ResourceType::new("aws", "secretsmanager", "Secret"),
        description: Some("A Secrets Manager secret".to_string()),
        input_properties: inputs,
        output_properties: outputs,
        required: vec!["name".to_string()],
    }
}
