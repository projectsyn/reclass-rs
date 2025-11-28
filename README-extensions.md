# reclass-rs Extensions

Initially, reclass-rs only implemented features present in [kapicorp-reclass], including extensions introduced by kapicorp-reclass.
This document covers extensions that are currently unique to reclass-rs.

## Non-compatible `compose_node_name` option

Reclass-rs supports the `compose_node_name` option.
By default, in contrast to kapicorp-reclass, reclass-rs preserves literal dots in composed node names.
For example, given the following inventory, the node's internal path is parsed as `['path', 'to', 'the.node']`:

```
.
├── classes
│   ├── cls1.yml
│   └── cls2.yml
└── nodes
    └── path
        └── to
            └── the.node.yml
```

However, optionally, reclass-rs can be configured to handle `compose_node_name` the same way that kapicorp-reclass does, by naively splitting node names on each dot.
To enable this compatibility mode, set `compat_flags: ['ComposeNodeNameLiteralDots']` in your inventory's `reclass-config.yml`.
In compatibility mode, the node's internal path for the previous inventory is `['path', 'to', 'the', 'node']`.

## Verbose warnings

Reclass-rs supports boolean config option `verbose_warnings`.
When verbose warnings are enabled, reclass-rs produces the following informational messages on standard error:

* warnings when dropping unrendered values which could potentially contain missing references.
* informational messages when replacing a missing reference with a default value.

## Default values for references

> [!IMPORTANT]
> This feature is currently experimental and may change at any time.

Reclass-rs supports specifying default (fallback) values for references.
The format for specifying a default value is

```
${some:reference:path::the_default_value}
```

We chose `::` as the separator between the reference path and the default value because `::` currently can't appear in references in valid kapicorp-reclass inventories (due to [kapicorp/kapitan#1171](https://github.com/kapicorp/kapitan/issues/1171)).

Reclass-rs first resolves nested references in the reference path and default value and then splits the reference contents into the path and default value.
References with default values can be used everywhere that references are supported, including class includes.

> [!NOTE]
> Currently, default values are only applied once all nested references have been successfully resolved.
> A missing nested reference without its own default value will result in an error even if the top-level reference specifies a default value.

For further processing, the default value is parsed as YAML.
Incomplete YAML flow values always raise an error (also for existing references that specify an incomplete YAML default value).

> [!NOTE]
> While it's generally possible to specify arbitrarily complex YAML default values inline, we recommend specifying complex default values through a nested reference.
> When providing complex default values inline, it may be necessary to carefully escape characters of the YAML value in order to ensure the reference parser keeps the YAML value intact.

### Example

```yaml
# nodes/test.yml
classes:
  - class1
  - class2

parameters:
  _base_directory: /tmp
```

```yaml
# classes/class1.yml
parameters:
  helm_values:
    a: a
    b: b
```

```yaml
# classes/class2.yml
parameters:
  _compile:
    jsonnet:
      - input_paths:
          - ${_base_directory}/foo.jsonnet
        type: jsonnet
        output_path: foo
    helm:
      - input_paths:
          - ${_base_directory}/charts/foo
        type: helm
        helm_values: ${helm_values}
        output_path: foo
    kustomize:
      - input_paths:
          - ${_base_directory}/kustomization.jsonnet
        type: jsonnet
        output_path: ${_base_directory}/kustomizations/foo
      - input_paths:
          - ${_base_directory}/kustomizations/foo/
        type: kustomize
        output_path: foo

  # Fall back to jsonnet rendering if method isn't configured
  compile: ${_compile:${method::jsonnet}}
```

The rendered parameters for node `test` shown above:

```yaml
parameters:
  _reclass_:
    environment: base
    name:
      full: test
      parts:
        - test
      path: test
      short: test
  _base_directory: /tmp
  _compile:
    helm:
      - helm_values:
          a: a
          b: b
        input_paths:
          - /tmp/charts/foo
        output_path: foo
        type: helm
    jsonnet:
      - input_paths:
          - /tmp/foo.jsonnet
        output_path: foo
        type: jsonnet
    kustomize:
      - input_paths:
          - /tmp/kustomization.jsonnet
        output_path: /tmp/kustomizations/foo
        type: jsonnet
      - input_paths:
          - /tmp/kustomizations/foo/
        output_path: foo
        type: kustomize
  compile:
    - input_paths:
        - /tmp/foo.jsonnet
      output_path: foo
      type: jsonnet
  helm_values:
    a: a
    b: b
```

> [!TIP]
> This example can also be found as node `n9` and classes `component.defaults` and `component.component` in this repository's `tests/inventory-reference-default-values`.

[kapicorp-reclass]: https://github.com/kapicorp/reclass
