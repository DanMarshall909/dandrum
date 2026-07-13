use std::fs;

use crate::diagnostics::error_codes;
use crate::graph::{PortDirection, SignalType};
use crate::kernel::{ChannelCount, StaticArg, StaticType, StaticValue};

use super::{load_kernel_patch_file, load_kernel_patch_str};

const COMPLETE_PATCH: &str = r#"
metadata:
  name: reusable_voice
  version: "1.2"
  author: Test Author
static_params:
  - name: channels
    type: int
    default: 2
  - name: mode
    type: enum
    default: clean
    allowed_values: [clean, driven]
  - name: source
    type: string
    default: "fn process(ctx) {}"
  - name: impulse
    type: resource
    default: room_ir
ports:
  - name: level
    direction: input
    signal: control
    channels: 1
    default: 0.75
    min: 0
    max: 1
    unit: linear
    maps_to: [amp.gain]
  - name: master
    direction: output
    signal: audio
    channels: $channels
    maps_from: [amp.audio_out]
module_definitions:
  - type: amplifier
    static_params:
      - name: channels
        type: int
        default: 1
      - name: mode
        type: enum
        allowed_values: [clean, driven]
      - name: label
        type: string
      - name: impulse
        type: resource
    ports:
      - name: gain
        direction: input
        signal: control
        channels: 1
        default: 1
        min: 0
        max: 2
        unit: linear
        maps_to: [inner.gain]
      - name: audio_out
        direction: output
        signal: audio
        channels: $channels
        maps_from: [inner.audio_out]
    modules:
      - id: inner
        type: gain
    connections: []
modules:
  - id: amp
    type: amplifier
    static:
      channels: $channels
      mode: driven
      label: main
      impulse: room_ir
    defaults:
      gain: 0.5
connections:
  - from: amp.audio_out
    to: amp.audio_in
"#;

#[test]
fn complete_kernel_document_produces_root_and_inline_graph_definitions() {
    let patch = load_kernel_patch_str(COMPLETE_PATCH).expect("kernel patch should load");

    assert_eq!(patch.metadata().name(), Some("reusable_voice"));
    assert_eq!(patch.metadata().version(), Some("1.2"));
    assert_eq!(patch.metadata().author(), Some("Test Author"));

    let root = patch.root();
    assert_eq!(root.name(), "reusable_voice");
    assert_eq!(root.static_params().len(), 4);
    assert_eq!(root.static_params()[0].static_type(), StaticType::Int);
    assert_eq!(
        root.static_params()[0].default(),
        Some(&StaticValue::Int(2))
    );
    assert_eq!(
        root.static_params()[1].allowed_values(),
        ["clean", "driven"]
    );
    assert_eq!(
        root.static_params()[2].default(),
        Some(&StaticValue::String("fn process(ctx) {}".into()))
    );
    assert_eq!(
        root.static_params()[3].default(),
        Some(&StaticValue::Resource("room_ir".into()))
    );

    let level = &root.ports()[0];
    assert_eq!(level.direction(), PortDirection::Input);
    assert_eq!(level.signal_type(), SignalType::Control);
    assert_eq!(level.channels(), &ChannelCount::Literal(1));
    let default = level.control_default().expect("control default");
    assert_eq!(default.default(), 0.75);
    assert_eq!(default.min(), Some(0.0));
    assert_eq!(default.max(), Some(1.0));
    assert_eq!(default.unit(), Some("linear"));
    assert_eq!(level.internal_targets()[0].node().as_str(), "amp");
    assert_eq!(level.internal_targets()[0].port(), "gain");

    let master = &root.ports()[1];
    assert_eq!(master.channels(), &ChannelCount::Param("channels".into()));
    assert_eq!(master.internal_sources()[0].node().as_str(), "amp");
    assert_eq!(
        root.nodes()[0].static_args()["channels"],
        StaticArg::ParamRef("channels".into())
    );
    assert_eq!(
        root.nodes()[0].static_args()["mode"],
        StaticArg::Literal(StaticValue::Enum("driven".into()))
    );
    assert_eq!(
        root.nodes()[0].static_args()["label"],
        StaticArg::Literal(StaticValue::String("main".into()))
    );
    assert_eq!(
        root.nodes()[0].static_args()["impulse"],
        StaticArg::Literal(StaticValue::Resource("room_ir".into()))
    );
    assert_eq!(root.nodes()[0].port_default_overrides()["gain"], 0.5);
    assert_eq!(root.connections().len(), 1);

    let composite = patch.registry().get("amplifier").expect("inline composite");
    assert_eq!(composite.static_params().len(), 4);
    assert_eq!(composite.ports().len(), 2);
    assert_eq!(composite.nodes().len(), 1);
}

#[test]
fn graph_declaration_has_patch_and_composite_symmetry() {
    let patch = load_kernel_patch_str(COMPLETE_PATCH).expect("kernel patch should load");
    let composite = patch.registry().get("amplifier").expect("inline composite");

    assert_eq!(
        patch.root().static_params()[0].name(),
        composite.static_params()[0].name()
    );
    assert_eq!(
        patch.root().static_params()[0].static_type(),
        composite.static_params()[0].static_type()
    );
    assert!(patch.root().ports()[0].control_default().is_some());
    assert!(composite.ports()[0].control_default().is_some());
    assert_eq!(composite.nodes()[0].definition_ref(), "gain");
}

#[test]
fn standalone_composite_shape_loads_as_a_root_patch() {
    let yaml = r#"
metadata: { name: amplifier }
static_params:
  - { name: channels, type: int, default: 1 }
ports:
  - { name: gain, direction: input, signal: control, channels: 1, default: 1, maps_to: inner.gain }
  - { name: audio_out, direction: output, signal: audio, channels: $channels, maps_from: inner.audio_out }
modules:
  - { id: inner, type: gain }
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("composite-shaped root should load");

    assert_eq!(patch.root().name(), "amplifier");
    assert_eq!(patch.root().static_params().len(), 1);
    assert_eq!(patch.root().ports().len(), 2);
    assert_eq!(patch.root().nodes().len(), 1);
}

#[test]
fn yaml_and_yml_files_load_through_public_file_surface() {
    let directory = std::env::temp_dir();
    for extension in ["yaml", "yml"] {
        let path = directory.join(format!("dandrum-kernel-document.{extension}"));
        fs::write(&path, COMPLETE_PATCH).expect("write test patch");
        let loaded = load_kernel_patch_file(&path).expect("load test patch");
        fs::remove_file(path).expect("remove test patch");
        assert_eq!(loaded.root().name(), "reusable_voice");
    }
}

#[test]
fn unsupported_file_format_has_structured_diagnostic() {
    let diagnostics = load_kernel_patch_file("patch.json").expect_err("JSON should be rejected");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_UNSUPPORTED_FORMAT
    );
    assert!(diagnostics.all()[0].message().contains(".json"));
}

#[test]
fn malformed_yaml_has_structured_diagnostic() {
    let diagnostics = load_kernel_patch_str("ports: [").expect_err("malformed YAML should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
}

#[test]
fn legacy_document_fields_are_rejected_with_specific_diagnostics() {
    for (field, code) in [
        ("render", error_codes::KERNEL_DOCUMENT_LEGACY_RENDER),
        (
            "voice_allocation",
            error_codes::KERNEL_DOCUMENT_LEGACY_VOICE_ALLOCATION,
        ),
    ] {
        let yaml = format!(
            "metadata: {{ name: test }}\nports: []\nmodules: []\nconnections: []\n{field}: {{}}\n"
        );
        let diagnostics = load_kernel_patch_str(&yaml).expect_err("legacy field should fail");
        assert_eq!(diagnostics.all()[0].error_code(), code);
        assert!(diagnostics.all()[0].message().contains(field));
    }
}

#[test]
fn legacy_instance_parameters_are_rejected_with_module_context() {
    let yaml = "metadata: { name: test }\nports: []\nmodules:\n  - id: amp\n    type: gain\n    parameters: { gain: 0.5 }\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("parameters should fail");
    let diagnostic = &diagnostics.all()[0];
    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_PARAMETERS
    );
    assert_eq!(diagnostic.module_id(), Some("amp"));
}

#[test]
fn legacy_parameter_binding_is_rejected_in_static_arguments() {
    let yaml = "metadata: { name: test }\nports: []\nmodule_definitions:\n  - type: child\n    static_params:\n      - { name: channels, type: int }\n    ports: []\n    modules: []\n    connections: []\nmodules:\n  - id: child\n    type: child\n    static: { channels: '${channels}' }\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("legacy binding should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_BINDING
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("child"));
}

#[test]
fn legacy_parameter_binding_is_rejected_in_default_overrides() {
    let yaml = "metadata: { name: test }\nports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - id: amp\n    type: gain\n    defaults: { gain: '${level}' }\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("legacy binding should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_BINDING
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("amp"));
}

#[test]
fn legacy_composite_asset_bindings_are_rejected_with_definition_context() {
    let yaml = "metadata: { name: test }\nports: []\nmodule_definitions:\n  - type: child\n    asset_bindings: []\n    ports: []\n    modules: []\n    connections: []\nmodules: []\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("asset bindings should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_ASSET_BINDINGS
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("child"));
}

#[test]
fn patch_without_root_output_has_structured_diagnostic() {
    let yaml = "metadata: { name: silent }\nports:\n  - { name: level, direction: input, signal: control, channels: 1, default: 0 }\nmodules: []\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("root output is required");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_NO_OUTPUT
    );
    assert!(
        diagnostics.all()[0]
            .message()
            .contains("no observable output")
    );
}

#[test]
fn unknown_static_and_default_names_are_reported_by_kernel_validation() {
    let yaml = r#"
metadata: { name: invalid_overrides }
ports:
  - { name: out, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }
module_definitions:
  - type: amplifier
    static_params:
      - { name: channels, type: int, default: 1 }
    ports:
      - { name: audio_out, direction: output, signal: audio, channels: $channels, maps_from: inner.audio_out }
    modules:
      - { id: inner, type: gain }
    connections: []
modules:
  - id: amp
    type: amplifier
    static: { unknown_shape: 2 }
  - id: amp_default
    type: amplifier
    defaults: { unknown_port: 0.5 }
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document shape should parse");
    let validation = patch.root().validate(patch.registry());
    let codes = validation
        .diagnostics()
        .all()
        .iter()
        .map(|diagnostic| diagnostic.error_code())
        .collect::<Vec<_>>();

    assert!(codes.contains(&error_codes::KERNEL_UNKNOWN_STATIC_ARGUMENT));
    assert!(codes.contains(&error_codes::KERNEL_OVERRIDE_UNKNOWN_PORT));
}
