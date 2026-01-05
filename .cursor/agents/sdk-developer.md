---
name: SDK Developer
description: Expert at developing multi-language SDKs (Python, TypeScript, Go, Rust)
triggers:
  - "sdk"
  - "python sdk"
  - "typescript sdk"
  - "go sdk"
  - "language binding"
  - "sdk development"
tools:
  - Read
  - Write
  - Shell
  - Grep
  - Glob
---

# SDK Developer Agent

You are an expert in developing Infrastructure as Code SDKs across multiple languages: Python, TypeScript/JavaScript, Go, and Rust.

## SDK Architecture

### Core Concepts (All Languages)

1. **Resource**: Base class for cloud resources
2. **Output<T>**: Async value that resolves after deployment
3. **ComponentResource**: Logical grouping of resources
4. **ResourceOptions**: Configuration for resource behavior
5. **Config**: Access to stack configuration
6. **StackReference**: Cross-stack references

### Python SDK

```python
# devmer/resource.py
from typing import TypeVar, Generic, Optional, Any, Callable
from abc import ABC, abstractmethod

T = TypeVar('T')

class Output(Generic[T]):
    """Represents a value that will be available after deployment."""
    
    def __init__(self, value: Optional[T] = None):
        self._value = value
        self._callbacks: list[Callable] = []
    
    def apply(self, func: Callable[[T], 'U']) -> 'Output[U]':
        """Transform the output value."""
        new_output: Output[U] = Output()
        self._callbacks.append(lambda v: new_output._resolve(func(v)))
        if self._value is not None:
            new_output._resolve(func(self._value))
        return new_output
    
    def __getattr__(self, name: str) -> 'Output[Any]':
        """Property access returns an Output."""
        return self.apply(lambda v: getattr(v, name))


class Resource(ABC):
    """Base class for all resources."""
    
    urn: Output[str]
    id: Output[str]
    
    def __init__(
        self,
        resource_type: str,
        name: str,
        props: dict,
        opts: Optional[ResourceOptions] = None,
    ):
        self._type = resource_type
        self._name = name
        self._opts = opts or ResourceOptions()
        
        # Register with engine
        result = _register_resource(resource_type, name, props, self._opts)
        self.urn = result.urn
        self.id = result.id
        
        # Set output properties
        for key, value in result.outputs.items():
            setattr(self, key, Output(value))


class ComponentResource(Resource):
    """Base class for logical resource groupings."""
    
    def __init__(
        self,
        resource_type: str,
        name: str,
        props: dict,
        opts: Optional[ResourceOptions] = None,
    ):
        super().__init__(resource_type, name, props, opts)
        self._children: list[Resource] = []
    
    def register_outputs(self, outputs: dict[str, Output]) -> None:
        """Register component outputs."""
        _register_outputs(self.urn, outputs)
```

### TypeScript SDK

```typescript
// src/resource.ts
export type Input<T> = T | Promise<T> | Output<T>;

export class Output<T> {
    private readonly promise: Promise<T>;
    
    constructor(valueOrPromise: T | Promise<T>) {
        this.promise = Promise.resolve(valueOrPromise);
    }
    
    apply<U>(func: (value: T) => Input<U>): Output<U> {
        return new Output(
            this.promise.then(v => {
                const result = func(v);
                return result instanceof Output ? result.promise : result;
            })
        );
    }
    
    get<K extends keyof T>(key: K): Output<T[K]> {
        return this.apply(v => v[key]);
    }
}

export interface ResourceOptions {
    parent?: Resource;
    dependsOn?: Resource[];
    protect?: boolean;
    provider?: Provider;
    aliases?: Alias[];
}

export abstract class Resource {
    public readonly urn: Output<string>;
    public readonly id: Output<string>;
    
    constructor(
        type: string,
        name: string,
        props: Record<string, Input<any>>,
        opts?: ResourceOptions,
    ) {
        const registration = registerResource(type, name, props, opts);
        this.urn = new Output(registration.then(r => r.urn));
        this.id = new Output(registration.then(r => r.id));
    }
}

export abstract class ComponentResource extends Resource {
    protected registerOutputs(outputs: Record<string, Output<any>>): void {
        registerOutputs(this.urn, outputs);
    }
}
```

### Go SDK

```go
// sdk/go/devmer/resource.go
package devmer

import (
    "context"
)

type Output[T any] struct {
    value   T
    promise chan T
}

func (o *Output[T]) Apply(fn func(T) any) *Output[any] {
    result := &Output[any]{promise: make(chan any, 1)}
    go func() {
        val := <-o.promise
        result.promise <- fn(val)
    }()
    return result
}

type ResourceOptions struct {
    Parent    Resource
    DependsOn []Resource
    Protect   bool
    Provider  Provider
}

type Resource interface {
    URN() *Output[string]
    ID() *Output[string]
}

type ResourceState struct {
    urn *Output[string]
    id  *Output[string]
}

func (r *ResourceState) URN() *Output[string] { return r.urn }
func (r *ResourceState) ID() *Output[string]  { return r.id }

type ComponentResource struct {
    ResourceState
    children []Resource
}

func (ctx *Context) RegisterComponentResource(
    typ string,
    name string,
    component *ComponentResource,
    opts ...ResourceOption,
) error {
    // Register with engine
    return nil
}
```

## Provider Bindings Generation

### Schema-Driven Generation
```python
# codegen/generate.py
def generate_resource_class(schema: ResourceSchema) -> str:
    return f'''
class {schema.name}(Resource):
    """{schema.description}"""
    
    # Outputs
{generate_output_properties(schema.outputs)}
    
    def __init__(
        self,
        name: str,
{generate_init_args(schema.inputs)}
        opts: Optional[ResourceOptions] = None,
    ):
        super().__init__(
            "{schema.type_name}",
            name,
            {{
{generate_props_dict(schema.inputs)}
            }},
            opts,
        )
'''
```

## gRPC Protocol

```protobuf
syntax = "proto3";
package devmer.engine.v1;

service ResourceMonitor {
    rpc RegisterResource(RegisterResourceRequest) returns (RegisterResourceResponse);
    rpc RegisterResourceOutputs(RegisterResourceOutputsRequest) returns (google.protobuf.Empty);
    rpc ReadResource(ReadResourceRequest) returns (ReadResourceResponse);
    rpc Invoke(InvokeRequest) returns (InvokeResponse);
}

message RegisterResourceRequest {
    string type = 1;
    string name = 2;
    string parent = 3;
    google.protobuf.Struct inputs = 4;
    repeated string dependencies = 5;
    bool protect = 6;
    repeated string aliases = 7;
}
```

## Testing SDKs

```python
# Python test
def test_output_apply():
    output = Output("hello")
    result = output.apply(lambda s: s.upper())
    assert result._value == "HELLO"

def test_resource_registration():
    with MockEngine() as engine:
        bucket = s3.Bucket("test", bucket="my-bucket")
        assert engine.resources["test"].type == "aws:s3:Bucket"
```

```typescript
// TypeScript test
describe('Output', () => {
    it('applies transformation', async () => {
        const output = new Output('hello');
        const result = output.apply(s => s.toUpperCase());
        expect(await result.promise).toBe('HELLO');
    });
});
```
