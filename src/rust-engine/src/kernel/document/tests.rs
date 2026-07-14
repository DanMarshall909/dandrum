use std::fs;

use crate::diagnostics::error_codes;
use crate::graph::{PortDirection, SignalType};
use crate::kernel::{
    ChannelCount, DefinitionImplementation, POLY_ALLOCATION_OLDEST_STEAL, POLY_DEFINITION,
    POLY_NOTE_EVENTS_INPUT, ResourceKind, ResourceOrigin, ResourceRef, StaticArg, StaticType,
    StaticValue,
};

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
    resource_kind: impulse_response
    default: { kind: impulse_response, path: room_ir.wav }
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
        resource_kind: impulse_response
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
      impulse: { kind: impulse_response, path: room_ir.wav }
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
        root.static_params()[3].static_type(),
        StaticType::Resource(ResourceKind::ImpulseResponse)
    );
    assert_eq!(
        root.static_params()[3].default(),
        Some(&StaticValue::Resource(ResourceRef::new(
            ResourceKind::ImpulseResponse,
            "room_ir.wav",
            ResourceOrigin::Document,
        )))
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
        StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
            ResourceKind::ImpulseResponse,
            "room_ir.wav",
            ResourceOrigin::Document,
        )))
    );
    assert_eq!(root.nodes()[0].port_default_overrides()["gain"], 0.5);
    assert_eq!(root.connections().len(), 1);

    let composite = patch.registry().get("amplifier").expect("inline composite");
    assert_eq!(composite.static_params().len(), 4);
    assert_eq!(composite.ports().len(), 2);
    assert_eq!(composite.nodes().len(), 1);
}

#[test]
fn resource_static_parameter_requires_a_resource_kind() {
    let error = load_kernel_patch_str(
        "metadata: { name: missing-kind }\nstatic_params:\n  - { name: sample, type: resource }\nports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules: []\nconnections: []\n",
    )
    .expect_err("resource declarations without a kind must fail");

    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
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

// --- 3.3 Input multiplicity YAML parsing ----------------------------------

#[test]
fn summing_multiplicity_parses_from_yaml() {
    let yaml = r#"
metadata:
  name: summing_test
ports:
  - { name: master, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: mixer
    ports:
      - { name: inputs, direction: input, signal: audio, channels: 1, multiplicity: summing }
      - { name: mix, direction: output, signal: audio, channels: 1 }
    modules: []
    connections: []
modules:
  - id: m
    type: mixer
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document should parse");
    let mixer = patch
        .registry()
        .get("mixer")
        .expect("mixer definition registered");
    let inputs = mixer
        .ports()
        .iter()
        .find(|p| p.name() == "inputs")
        .expect("inputs port exists");
    assert_eq!(
        inputs.multiplicity(),
        crate::kernel::Multiplicity::Summing,
        "YAML multiplicity: summing should parse"
    );
}

#[test]
fn omitted_multiplicity_defaults_to_single_source() {
    let yaml = r#"
metadata:
  name: single_test
ports:
  - { name: master, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: gain_like
    ports:
      - { name: audio_in, direction: input, signal: audio, channels: 1 }
      - { name: audio_out, direction: output, signal: audio, channels: 1 }
    modules: []
    connections: []
modules:
  - id: g
    type: gain_like
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document should parse");
    let gain_like = patch
        .registry()
        .get("gain_like")
        .expect("gain_like definition registered");
    let audio_in = gain_like
        .ports()
        .iter()
        .find(|p| p.name() == "audio_in")
        .expect("audio_in port exists");
    assert_eq!(
        audio_in.multiplicity(),
        crate::kernel::Multiplicity::SingleSource,
        "omitted multiplicity should default to single_source"
    );
}

#[test]
fn single_source_multiplicity_parses_explicitly() {
    let yaml = r#"
metadata:
  name: explicit_single
ports:
  - { name: master, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: single_input
    ports:
      - { name: audio_in, direction: input, signal: audio, channels: 1, multiplicity: single_source }
      - { name: audio_out, direction: output, signal: audio, channels: 1 }
    modules: []
    connections: []
modules:
  - id: s
    type: single_input
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document should parse");
    let single_input = patch
        .registry()
        .get("single_input")
        .expect("single_input definition registered");
    let audio_in = single_input
        .ports()
        .iter()
        .find(|p| p.name() == "audio_in")
        .expect("audio_in port exists");
    assert_eq!(
        audio_in.multiplicity(),
        crate::kernel::Multiplicity::SingleSource,
        "explicit single_source should parse"
    );
}

const SCRIPT_DEFINITION_PATCH: &str = r#"
metadata: { name: scripted }
ports:
  - { name: out, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: counter
    implementation: script
    static_params:
      - { name: language, type: enum, default: rhai, allowed_values: [rhai] }
      - { name: source, type: string }
    ports:
      - { name: events, direction: input, signal: event, channels: 1 }
      - { name: increment, direction: input, signal: control, channels: 1, default: 1 }
      - { name: count, direction: output, signal: control, channels: 1 }
modules:
  - id: first
    type: counter
    static: { source: "fn process(ctx) {}" }
    defaults: { increment: 2 }
  - id: second
    type: counter
    static: { source: "fn process(ctx) {}" }
connections: []
"#;

#[test]
fn script_backed_definition_uses_the_ordinary_node_shape() {
    let patch = load_kernel_patch_str(SCRIPT_DEFINITION_PATCH).expect("script definition loads");
    let definition = patch.registry().get("counter").expect("named definition");

    assert_eq!(
        definition.implementation(),
        DefinitionImplementation::Script
    );
    assert_eq!(
        definition.static_params()[0].static_type(),
        StaticType::Enum
    );
    assert_eq!(
        definition.static_params()[1].static_type(),
        StaticType::String
    );
    assert_eq!(definition.ports().len(), 3);
    assert_eq!(patch.root().nodes()[0].definition_ref(), "counter");
    assert_eq!(
        patch.root().nodes()[0].port_default_overrides()["increment"],
        2.0
    );
}

#[test]
fn script_instances_reject_ad_hoc_port_fields() {
    let yaml = SCRIPT_DEFINITION_PATCH.replace(
        "    defaults: { increment: 2 }",
        "    defaults: { increment: 2 }\n    inputs: [{ name: invented, signal: control }]",
    );

    let diagnostics = load_kernel_patch_str(&yaml).expect_err("instance ports are not authorable");

    assert_eq!(
        diagnostics.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
    assert!(diagnostics.all()[0].message().contains("inputs"));
}

#[test]
fn script_definition_rejects_internal_graph_structure_and_audio_ports() {
    for (replacement, expected_fragment) in [
        (
            "    implementation: script\n    modules: [{ id: inner, type: gain }]",
            "internal modules",
        ),
        (
            "    implementation: script\n    connections: [{ from: a.out, to: b.in }]",
            "internal connections",
        ),
        (
            "      - { name: count, direction: output, signal: audio, channels: 1 }",
            "audio port",
        ),
    ] {
        let yaml = if replacement.starts_with("    implementation") {
            SCRIPT_DEFINITION_PATCH.replace("    implementation: script", replacement)
        } else {
            SCRIPT_DEFINITION_PATCH.replace(
                "      - { name: count, direction: output, signal: control, channels: 1 }",
                replacement,
            )
        };

        let diagnostics =
            load_kernel_patch_str(&yaml).expect_err("malformed script definition must fail");
        let diagnostic = diagnostics.errors().next().unwrap();
        assert_eq!(
            diagnostic.error_code(),
            error_codes::KERNEL_SCRIPT_DEFINITION_INVALID
        );
        assert!(diagnostic.message().contains(expected_fragment));
        assert_eq!(diagnostic.module_id(), Some("counter"));
    }
}

#[test]
fn script_definition_requires_typed_language_and_source_declarations() {
    for (yaml, expected_fragment) in [
        (
            SCRIPT_DEFINITION_PATCH.replace(
                "      - { name: language, type: enum, default: rhai, allowed_values: [rhai] }\n",
                "",
            ),
            "language",
        ),
        (
            SCRIPT_DEFINITION_PATCH.replace("      - { name: source, type: string }\n", ""),
            "source",
        ),
        (
            SCRIPT_DEFINITION_PATCH
                .replace("name: language, type: enum", "name: language, type: string"),
            "language",
        ),
        (
            SCRIPT_DEFINITION_PATCH
                .replace("name: source, type: string", "name: source, type: int"),
            "source",
        ),
    ] {
        let diagnostics =
            load_kernel_patch_str(&yaml).expect_err("script construction declarations are fixed");
        let diagnostic = diagnostics.errors().next().unwrap();
        assert_eq!(
            diagnostic.error_code(),
            error_codes::KERNEL_SCRIPT_DEFINITION_INVALID
        );
        assert!(diagnostic.message().contains(expected_fragment));
        assert_eq!(diagnostic.module_id(), Some("counter"));
    }
}

#[test]
fn unsupported_definition_implementation_has_a_structured_diagnostic() {
    let yaml = SCRIPT_DEFINITION_PATCH.replace("implementation: script", "implementation: wasm");

    let diagnostics = load_kernel_patch_str(&yaml).expect_err("unsupported implementation fails");
    let diagnostic = diagnostics.errors().next().unwrap();

    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_DEFINITION_IMPLEMENTATION_UNSUPPORTED
    );
    assert_eq!(diagnostic.module_id(), Some("counter"));
    assert_eq!(diagnostic.actual(), Some("wasm"));
}

const POLY_PATCH: &str = r#"
metadata: { name: poly_patch }
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: voices.audio }
module_definitions:
  - type: voice
    ports:
      - { name: level, direction: input, signal: control, channels: 1, default: 0.5 }
      - { name: audio, direction: output, signal: audio, channels: 2 }
    modules: []
    connections: []
modules:
  - { id: midi, type: midi_input }
  - id: voices
    type: poly
    static: { definition: voice, max_voices: 8, allocation: oldest-steal }
connections:
  - { from: midi.events, to: voices.notes }
"#;

#[test]
fn yaml_poly_node_uses_ordinary_node_shape_and_synthesized_interface() {
    let patch = load_kernel_patch_str(POLY_PATCH).expect("poly YAML loads");
    let poly = &patch.root().nodes()[1];

    assert_eq!(poly.definition_ref(), POLY_DEFINITION);
    assert_eq!(
        poly.static_args()["allocation"],
        StaticArg::Literal(StaticValue::Enum(POLY_ALLOCATION_OLDEST_STEAL.to_string()))
    );
    assert!(
        patch.root().validate(patch.registry()).is_ok(),
        "YAML connections validate through the synthesized interface"
    );
    let ports = patch
        .root()
        .resolved_node_ports(patch.registry(), poly.id())
        .expect("poly ports resolve");
    assert!(
        ports
            .iter()
            .any(|port| port.name() == POLY_NOTE_EVENTS_INPUT)
    );
    assert!(ports.iter().any(|port| port.name() == "level"));
    assert!(ports.iter().any(|port| port.name() == "audio"));
}

#[test]
fn yaml_poly_rejects_invalid_allocation_policy_during_validation() {
    let yaml = POLY_PATCH.replace("oldest-steal", "newest-steal");
    let patch = load_kernel_patch_str(&yaml).expect("document shape still loads");
    let validation = patch.root().validate(patch.registry());

    assert_eq!(
        validation
            .diagnostics()
            .errors()
            .next()
            .unwrap()
            .error_code(),
        error_codes::KERNEL_STATIC_ARGUMENT_INVALID_ENUM_VALUE
    );
}
