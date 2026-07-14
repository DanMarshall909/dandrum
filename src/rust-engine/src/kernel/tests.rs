use super::*;
use crate::diagnostics::error_codes;

/// Build a `gain` primitive definition: one audio input, one control `level`
/// input with a default, one audio output. Used across signal/default tests.
fn gain_primitive() -> GraphDefinition {
    GraphDefinition::new("gain")
        .with_port(Port::input("audio_in", SignalType::Audio, 1))
        .with_port(
            Port::input("level", SignalType::Control, 1)
                .with_control_default(ControlDefault::new(1.0).with_min(0.0).with_max(2.0)),
        )
        .with_port(Port::output("audio_out", SignalType::Audio, 1))
}

/// Build an `echo` primitive that is channel-polymorphic via a `channels`
/// integer static parameter defaulting to `2`.
fn echo_primitive() -> GraphDefinition {
    GraphDefinition::new("echo")
        .with_static_param(
            StaticParam::new("channels", StaticType::Int).with_default(StaticValue::Int(2)),
        )
        .with_port(Port::input(
            "audio_in",
            SignalType::Audio,
            ChannelCount::param("channels"),
        ))
        .with_port(Port::output(
            "audio_out",
            SignalType::Audio,
            ChannelCount::param("channels"),
        ))
}

fn only_error_code(validation: &KernelValidation) -> &str {
    let errors: Vec<_> = validation.diagnostics().errors().collect();
    assert_eq!(
        errors.len(),
        1,
        "expected exactly one error, got: {:?}",
        validation.diagnostics()
    );
    errors[0].error_code()
}

fn poly_node(id: &str, definition: &str, max_voices: i64, allocation: &str) -> Node {
    Node::new(NodeId::new(id), POLY_DEFINITION)
        .with_static_arg(
            POLY_WRAPPED_DEFINITION_PARAM,
            StaticArg::Literal(StaticValue::String(definition.to_string())),
        )
        .with_static_arg(
            POLY_MAX_VOICES_PARAM,
            StaticArg::Literal(StaticValue::Int(max_voices)),
        )
        .with_static_arg(
            POLY_ALLOCATION_PARAM,
            StaticArg::Literal(StaticValue::Enum(allocation.to_string())),
        )
}

fn poly_registry(voice: GraphDefinition) -> DefinitionRegistry {
    builtins::builtin_registry().with_definition(voice)
}

// --- 1.1 Kernel types: construction and equality -------------------------

#[test]
fn graph_definition_preserves_declared_structure() {
    let definition = GraphDefinition::new("voice")
        .with_static_param(StaticParam::new("channels", StaticType::Int))
        .with_port(Port::output("audio_out", SignalType::Audio, 2))
        .with_node(Node::new(NodeId::new("osc"), "oscillator"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("osc"), "audio"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    assert_eq!(definition.name(), "voice");
    assert_eq!(definition.static_params().len(), 1);
    assert_eq!(definition.ports()[0].name(), "audio_out");
    assert_eq!(definition.nodes()[0].id().as_str(), "osc");
    assert_eq!(definition.connections()[0].source().port(), "audio");
}

#[test]
fn node_records_definition_reference_static_args_and_overrides() {
    let node = Node::new(NodeId::new("delay"), "echo")
        .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2)))
        .with_default_override("feedback", 0.5);

    assert_eq!(node.definition_ref(), "echo");
    assert_eq!(
        node.static_args().get("channels"),
        Some(&StaticArg::Literal(StaticValue::Int(2)))
    );
    assert_eq!(node.port_default_overrides().get("feedback"), Some(&0.5));
}

#[test]
fn port_carries_channel_count_and_control_default_metadata() {
    let port = Port::input("cutoff", SignalType::Control, 1).with_control_default(
        ControlDefault::new(440.0)
            .with_min(20.0)
            .with_max(20_000.0)
            .with_unit("hz"),
    );

    assert_eq!(port.channels(), &ChannelCount::Literal(1));
    let control = port.control_default().expect("control port has default");
    assert_eq!(control.default(), 440.0);
    assert_eq!(control.min(), Some(20.0));
    assert_eq!(control.unit(), Some("hz"));
}

#[test]
fn identical_definitions_are_equal_and_differing_ones_are_not() {
    let build = || {
        GraphDefinition::new("voice")
            .with_port(Port::output("audio_out", SignalType::Audio, 2))
            .with_node(Node::new(NodeId::new("osc"), "oscillator"))
    };

    assert_eq!(build(), build());
    assert_ne!(
        build(),
        build().with_node(Node::new(NodeId::new("extra"), "gain"))
    );
}

// --- 1.2 Static parameters ----------------------------------------------

#[test]
fn static_parameter_declaration_is_preserved_for_resolution_and_discovery() {
    let param = StaticParam::new("channels", StaticType::Int).with_default(StaticValue::Int(2));

    assert_eq!(param.name(), "channels");
    assert_eq!(param.static_type(), StaticType::Int);
    assert_eq!(param.default(), Some(&StaticValue::Int(2)));
}

#[test]
fn string_static_parameter_preserves_inline_script_source() {
    let source = "fn process(input) { input }";
    let script = GraphDefinition::new("script").with_static_param(
        StaticParam::new("source", StaticType::String)
            .with_default(StaticValue::String(source.to_string())),
    );
    let registry = DefinitionRegistry::new().with_definition(script);
    let root = GraphDefinition::new("root").with_node(Node::new(NodeId::new("script"), "script"));

    let flat = root.flatten(&registry).expect("script source resolves");
    let resolved = flat.nodes()[0].static_args();

    assert_eq!(
        resolved.get("source"),
        Some(&StaticValue::String(source.to_string()))
    );
}

#[test]
fn connection_targeting_a_static_parameter_is_rejected_as_not_a_port() {
    let registry = DefinitionRegistry::new()
        .with_definition(echo_primitive())
        .with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("g"), "gain"))
        .with_node(Node::new(NodeId::new("e"), "echo"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("g"), "audio_out"),
            PortRef::new(NodeId::new("e"), "channels"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_STATIC_PARAM_NOT_A_PORT
    );
}

#[test]
fn missing_required_static_argument_is_rejected() {
    let synth = GraphDefinition::new("synth")
        .with_static_param(StaticParam::new("voices", StaticType::Int));
    let registry = DefinitionRegistry::new().with_definition(synth);
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("s"), "synth"));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_MISSING_STATIC_ARGUMENT
    );
}

#[test]
fn static_argument_type_mismatch_is_rejected() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo").with_static_arg(
            "channels",
            StaticArg::Literal(StaticValue::Enum("stereo".to_string())),
        ),
    );

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_STATIC_ARGUMENT_TYPE_MISMATCH
    );
}

#[test]
fn unknown_static_argument_name_is_rejected() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("voices", StaticArg::Literal(StaticValue::Int(4))),
    );

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_UNKNOWN_STATIC_ARGUMENT
    );
}

#[test]
fn static_argument_expression_is_rejected() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo").with_static_arg(
            "channels",
            StaticArg::Expression("$channels + 1".to_string()),
        ),
    );

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_STATIC_ARGUMENT_EXPRESSION
    );
}

#[test]
fn static_argument_name_pass_through_resolves_from_enclosing_definition() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    // A composite declares `channels` (default 4) and forwards it by name.
    let composite = GraphDefinition::new("stereo_bank")
        .with_static_param(
            StaticParam::new("channels", StaticType::Int).with_default(StaticValue::Int(4)),
        )
        .with_node(
            Node::new(NodeId::new("e"), "echo")
                .with_static_arg("channels", StaticArg::ParamRef("channels".to_string())),
        );

    let validation = composite.validate(&registry);
    assert!(
        validation.is_ok(),
        "name pass-through should resolve: {:?}",
        validation.diagnostics()
    );

    let ports = composite
        .resolved_node_ports(&registry, &NodeId::new("e"))
        .expect("node ports resolve");
    assert_eq!(ports[0].channels(), 4);
}

#[test]
fn unknown_static_parameter_reference_is_rejected() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let composite = GraphDefinition::new("bank").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::ParamRef("missing".to_string())),
    );

    let validation = composite.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_UNKNOWN_STATIC_PARAM_REFERENCE
    );
}

// --- 1.3 Channel-count resolution and validation ------------------------

#[test]
fn one_definition_serves_mono_and_stereo_via_static_channel_count() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(
            Node::new(NodeId::new("mono"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(1))),
        )
        .with_node(
            Node::new(NodeId::new("stereo"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        );

    let validation = definition.validate(&registry);
    assert!(validation.is_ok(), "both instances validate");

    let mono = definition
        .resolved_node_ports(&registry, &NodeId::new("mono"))
        .expect("mono resolves");
    let stereo = definition
        .resolved_node_ports(&registry, &NodeId::new("stereo"))
        .expect("stereo resolves");
    assert_eq!(mono[0].channels(), 1);
    assert_eq!(stereo[0].channels(), 2);
}

#[test]
fn matching_channel_counts_connect() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(
            Node::new(NodeId::new("a"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        )
        .with_node(
            Node::new(NodeId::new("b"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        )
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("b"), "audio_in"),
        ));

    assert!(definition.validate(&registry).is_ok());
}

#[test]
fn mismatched_channel_counts_are_rejected_reporting_both_counts() {
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(
            Node::new(NodeId::new("a"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
        )
        .with_node(
            Node::new(NodeId::new("b"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(1))),
        )
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("b"), "audio_in"),
        ));

    let validation = definition.validate(&registry);
    let errors: Vec<_> = validation.diagnostics().errors().collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].error_code(),
        error_codes::KERNEL_CHANNEL_COUNT_MISMATCH
    );
    assert_eq!(errors[0].expected(), Some("2"));
    assert_eq!(errors[0].actual(), Some("1"));
}

// --- 1.4 Signal-type compatibility and promotion ------------------------

#[test]
fn control_output_promotes_to_audio_input_and_records_a_promotion_step() {
    let control_source =
        GraphDefinition::new("lfo").with_port(Port::output("out", SignalType::Control, 1));
    let registry = DefinitionRegistry::new()
        .with_definition(control_source)
        .with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("lfo"), "lfo"))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("lfo"), "out"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let validation = definition.validate(&registry);

    assert!(validation.is_ok(), "control→audio is legal");
    assert_eq!(validation.promotions().len(), 1);
    let promotion = &validation.promotions()[0];
    assert_eq!(
        promotion.source(),
        &PortRef::new(NodeId::new("lfo"), "out"),
        "the promotion records the control port it converts from"
    );
    assert_eq!(promotion.destination().port(), "audio_in");
    assert_eq!(
        promotion.channels(),
        1,
        "the promotion adopts the audio destination's width"
    );
}

#[test]
fn audio_output_cannot_feed_control_input() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("b"), "level"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_INCOMPATIBLE_SIGNAL_TYPES
    );
}

#[test]
fn event_ports_never_convert_to_audio() {
    let gate =
        GraphDefinition::new("gate").with_port(Port::output("trigger", SignalType::Event, 1));
    let registry = DefinitionRegistry::new()
        .with_definition(gate)
        .with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("g"), "gate"))
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("g"), "trigger"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_INCOMPATIBLE_SIGNAL_TYPES
    );
}

// --- 1.5 Unconnected-input default resolution ---------------------------

#[test]
fn unconnected_control_input_uses_declared_default() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    let effective = definition.effective_control_input(&registry, &NodeId::new("amp"), "level");

    assert_eq!(effective, Some(EffectiveInput::Value(1.0)));
}

#[test]
fn instance_override_replaces_declared_default() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("amp"), "gain").with_default_override("level", 0.25));

    let effective = definition.effective_control_input(&registry, &NodeId::new("amp"), "level");

    assert_eq!(effective, Some(EffectiveInput::Value(0.25)));
}

#[test]
fn incoming_connection_takes_precedence_over_default_and_override() {
    let control_source =
        GraphDefinition::new("lfo").with_port(Port::output("out", SignalType::Control, 1));
    let registry = DefinitionRegistry::new()
        .with_definition(gain_primitive())
        .with_definition(control_source);
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("lfo"), "lfo"))
        .with_node(Node::new(NodeId::new("amp"), "gain").with_default_override("level", 0.25))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("lfo"), "out"),
            PortRef::new(NodeId::new("amp"), "level"),
        ));

    let effective = definition.effective_control_input(&registry, &NodeId::new("amp"), "level");

    assert_eq!(effective, Some(EffectiveInput::Connected));
}

#[test]
fn override_of_unknown_port_is_rejected() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("amp"), "gain").with_default_override("resonance", 0.5));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_OVERRIDE_UNKNOWN_PORT
    );
}

// --- Endpoint resolution diagnostics ------------------------------------

#[test]
fn connection_to_missing_node_is_rejected() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("amp"), "audio_out"),
            PortRef::new(NodeId::new("ghost"), "audio_in"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_MISSING_NODE
    );
}

#[test]
fn connection_to_a_port_the_definition_does_not_declare_reports_a_missing_port() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("b"), "sidechain_in"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_MISSING_PORT
    );
}

#[test]
fn validation_rejects_an_out_of_range_resolved_channel_count_without_resolving_ports() {
    // `echo` resolves its port widths from `channels`; a zero channel count must
    // be reported rather than silently falling back to a one-channel port.
    let registry = DefinitionRegistry::new().with_definition(echo_primitive());
    let definition = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo")
            .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(0))),
    );

    let validation = definition.validate(&registry);

    let errors: Vec<_> = validation.diagnostics().errors().collect();
    assert!(
        errors
            .iter()
            .all(|d| d.error_code() == error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE),
        "expected only invalid-static-reference-value errors, got: {errors:?}"
    );
    let ports: Vec<_> = errors.iter().filter_map(|d| d.port_name()).collect();
    assert_eq!(
        ports,
        ["audio_in", "audio_out"],
        "every port whose width resolves from the bad parameter is named"
    );
}

#[test]
fn connection_from_a_node_whose_ports_failed_to_resolve_adds_no_second_diagnostic() {
    // `e`'s ports never resolve (channel count of zero). The connection out of
    // it must not also be reported as a missing port: the root cause is enough.
    let registry = DefinitionRegistry::new()
        .with_definition(echo_primitive())
        .with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(
            Node::new(NodeId::new("e"), "echo")
                .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(0))),
        )
        .with_node(Node::new(NodeId::new("amp"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("e"), "audio_out"),
            PortRef::new(NodeId::new("amp"), "audio_in"),
        ));

    let validation = definition.validate(&registry);

    assert!(
        validation
            .diagnostics()
            .errors()
            .all(|d| d.error_code() == error_codes::KERNEL_INVALID_STATIC_REFERENCE_VALUE),
        "only the unresolved channel count is reported, got: {:?}",
        validation.diagnostics()
    );
}

#[test]
fn validation_accepts_an_override_of_a_port_the_definition_declares() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("amp"), "gain").with_default_override("level", 0.5));

    let validation = definition.validate(&registry);

    assert!(
        validation.is_ok(),
        "overriding a declared control port is legal, got: {:?}",
        validation.diagnostics()
    );
}

#[test]
fn control_to_audio_promotion_with_mismatched_channel_counts_records_no_promotion() {
    // Promotion is only legal once the widths agree: a mono control output
    // cannot silently fan out into a stereo audio input.
    let control_source =
        GraphDefinition::new("lfo").with_port(Port::output("out", SignalType::Control, 1));
    let registry = DefinitionRegistry::new()
        .with_definition(control_source)
        .with_definition(echo_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("lfo"), "lfo"))
        .with_node(Node::new(NodeId::new("e"), "echo"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("lfo"), "out"),
            PortRef::new(NodeId::new("e"), "audio_in"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_CHANNEL_COUNT_MISMATCH
    );
    assert!(
        validation.promotions().is_empty(),
        "a rejected connection records no promotion step"
    );
}

// --- Unresolved lookups on the discovery surface -------------------------

#[test]
fn resolved_node_ports_returns_none_for_an_unknown_node() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    assert_eq!(
        definition.resolved_node_ports(&registry, &NodeId::new("ghost")),
        None,
        "a node the definition does not declare has no resolved ports"
    );
}

#[test]
fn resolved_node_ports_returns_none_when_the_definition_is_unknown() {
    let registry = DefinitionRegistry::new();
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    assert_eq!(
        definition.resolved_node_ports(&registry, &NodeId::new("amp")),
        None,
        "an unregistered definition yields no invented ports"
    );
}

#[test]
fn resolved_node_ports_returns_none_when_a_required_static_argument_is_missing() {
    // `channels` has no default here, so the node cannot resolve its port widths.
    let echo = GraphDefinition::new("echo")
        .with_static_param(StaticParam::new("channels", StaticType::Int))
        .with_port(Port::input(
            "audio_in",
            SignalType::Audio,
            ChannelCount::param("channels"),
        ));
    let registry = DefinitionRegistry::new().with_definition(echo);
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("e"), "echo"));

    assert_eq!(
        definition.resolved_node_ports(&registry, &NodeId::new("e")),
        None,
        "an unsupplied static argument leaves the port widths unresolved"
    );
}

#[test]
fn effective_control_input_returns_none_for_an_unknown_node() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    assert_eq!(
        definition.effective_control_input(&registry, &NodeId::new("ghost"), "level"),
        None
    );
}

#[test]
fn effective_control_input_returns_none_when_the_definition_is_unknown() {
    let registry = DefinitionRegistry::new();
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    assert_eq!(
        definition.effective_control_input(&registry, &NodeId::new("amp"), "level"),
        None
    );
}

#[test]
fn effective_control_input_returns_none_for_a_port_that_is_not_a_control_input() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("amp"), "gain"));

    assert_eq!(
        definition.effective_control_input(&registry, &NodeId::new("amp"), "audio_in"),
        None,
        "an audio input is not a control input"
    );
    assert_eq!(
        definition.effective_control_input(&registry, &NodeId::new("amp"), "missing"),
        None,
        "a port the definition does not declare has no effective input"
    );
}

#[test]
fn connection_from_input_port_reports_incorrect_direction() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_connection(Connection::new(
            // audio_in is an input port; using it as a source is wrong direction.
            PortRef::new(NodeId::new("a"), "audio_in"),
            PortRef::new(NodeId::new("b"), "audio_in"),
        ));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_INCORRECT_PORT_DIRECTION
    );
}

// --- Static references must resolve (no silent channel/latency fallback) --

#[test]
fn dangling_channel_reference_fails_compilation_loudly() {
    // `echo` references a `channels` static parameter it never declares.
    let broken = GraphDefinition::new("echo").with_port(Port::input(
        "audio_in",
        SignalType::Audio,
        ChannelCount::param("channels"),
    ));
    let registry = DefinitionRegistry::new().with_definition(broken);
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("e"), "echo"));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_UNRESOLVED_STATIC_REFERENCE
    );
}

#[test]
fn channel_reference_to_non_integer_static_param_fails_compilation() {
    let broken = GraphDefinition::new("echo")
        .with_static_param(StaticParam::new("channels", StaticType::Enum))
        .with_port(Port::input(
            "audio_in",
            SignalType::Audio,
            ChannelCount::param("channels"),
        ));
    let registry = DefinitionRegistry::new().with_definition(broken);
    let definition = GraphDefinition::new("root").with_node(
        Node::new(NodeId::new("e"), "echo").with_static_arg(
            "channels",
            StaticArg::Literal(StaticValue::Enum("wide".into())),
        ),
    );

    let validation = definition.validate(&registry);

    assert!(
        validation
            .diagnostics()
            .errors()
            .any(|d| d.error_code() == error_codes::KERNEL_UNRESOLVED_STATIC_REFERENCE),
        "non-integer channel reference must fail loudly: {:?}",
        validation.diagnostics()
    );
}

#[test]
fn dangling_channel_reference_makes_resolved_ports_unavailable() {
    let broken = GraphDefinition::new("echo").with_port(Port::input(
        "audio_in",
        SignalType::Audio,
        ChannelCount::param("channels"),
    ));
    let registry = DefinitionRegistry::new().with_definition(broken);
    let definition = GraphDefinition::new("root").with_node(Node::new(NodeId::new("e"), "echo"));

    assert_eq!(
        definition.resolved_node_ports(&registry, &NodeId::new("e")),
        None,
        "query must refuse to invent a channel count for a dangling reference"
    );
}

// --- 2.3 Feedback cycles require a feedback_delay node -------------------

/// A `feedback_delay` primitive carrying ports of the given signal type.
fn feedback_delay(signal_type: SignalType) -> GraphDefinition {
    GraphDefinition::new(FEEDBACK_DELAY_DEFINITION)
        .with_port(Port::input("in", signal_type, 1))
        .with_port(Port::output("out", signal_type, 1))
}

/// An ordinary in/out primitive of the given signal type that does NOT
/// legalize a cycle.
fn passthrough(name: &str, signal_type: SignalType) -> GraphDefinition {
    GraphDefinition::new(name)
        .with_port(Port::input("in", signal_type, 1))
        .with_port(Port::output("out", signal_type, 1))
}

fn two_node_cycle(root: &str, first: &str, second: &str) -> GraphDefinition {
    GraphDefinition::new(root)
        .with_node(Node::new(NodeId::new("a"), first))
        .with_node(Node::new(NodeId::new("b"), second))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "out"),
            PortRef::new(NodeId::new("b"), "in"),
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("b"), "out"),
            PortRef::new(NodeId::new("a"), "in"),
        ))
}

#[test]
fn audio_feedback_through_feedback_delay_is_valid() {
    let registry = DefinitionRegistry::new()
        .with_definition(passthrough("gain", SignalType::Audio))
        .with_definition(feedback_delay(SignalType::Audio));
    let definition = two_node_cycle("root", "gain", FEEDBACK_DELAY_DEFINITION);

    let validation = definition.validate(&registry);

    assert!(
        validation.is_ok(),
        "cycle through feedback_delay is legal: {:?}",
        validation.diagnostics()
    );
}

#[test]
fn instantaneous_audio_feedback_is_rejected_naming_the_required_primitive() {
    let registry =
        DefinitionRegistry::new().with_definition(passthrough("gain", SignalType::Audio));
    let definition = two_node_cycle("root", "gain", "gain");

    let validation = definition.validate(&registry);

    let errors: Vec<_> = validation.diagnostics().errors().collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].error_code(),
        error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY
    );
    assert!(
        errors[0].message().contains(FEEDBACK_DELAY_DEFINITION),
        "diagnostic names the required primitive"
    );
}

#[test]
fn ordinary_delay_module_does_not_legalize_a_cycle() {
    let registry = DefinitionRegistry::new()
        .with_definition(passthrough("gain", SignalType::Audio))
        .with_definition(passthrough("delay", SignalType::Audio));
    let definition = two_node_cycle("root", "gain", "delay");

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY
    );
}

#[test]
fn control_feedback_through_feedback_delay_is_valid() {
    let registry = DefinitionRegistry::new()
        .with_definition(passthrough("mod", SignalType::Control))
        .with_definition(feedback_delay(SignalType::Control));
    let definition = two_node_cycle("root", "mod", FEEDBACK_DELAY_DEFINITION);

    assert!(definition.validate(&registry).is_ok());
}

#[test]
fn instantaneous_control_feedback_is_rejected() {
    let registry =
        DefinitionRegistry::new().with_definition(passthrough("mod", SignalType::Control));
    let definition = two_node_cycle("root", "mod", "mod");

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY
    );
}

#[test]
fn node_referencing_unknown_definition_is_rejected() {
    let registry = DefinitionRegistry::new();
    let definition =
        GraphDefinition::new("root").with_node(Node::new(NodeId::new("x"), "nonexistent"));

    let validation = definition.validate(&registry);

    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_UNKNOWN_DEFINITION
    );
}

// --- 3.3 Input multiplicity ----------------------------------------------

fn summing_mixer_primitive() -> GraphDefinition {
    GraphDefinition::new("mixer")
        .with_port(
            Port::input("inputs", SignalType::Audio, 1).with_multiplicity(Multiplicity::Summing),
        )
        .with_port(Port::output("mix", SignalType::Audio, 1))
}

#[test]
fn single_source_input_rejects_multiple_connections() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_node(Node::new(NodeId::new("c"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("c"), "audio_in"),
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("b"), "audio_out"),
            PortRef::new(NodeId::new("c"), "audio_in"),
        ));

    let validation = definition.validate(&registry);
    assert_eq!(
        only_error_code(&validation),
        error_codes::KERNEL_MULTIPLE_SOURCES
    );
}

#[test]
fn summing_input_accepts_multiple_connections() {
    let registry = DefinitionRegistry::new()
        .with_definition(gain_primitive())
        .with_definition(summing_mixer_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_node(Node::new(NodeId::new("mix"), "mixer"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("mix"), "inputs"),
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("b"), "audio_out"),
            PortRef::new(NodeId::new("mix"), "inputs"),
        ));

    let validation = definition.validate(&registry);
    assert!(
        validation.is_ok(),
        "summing input should accept multiple connections: {:?}",
        validation.diagnostics()
    );
}

#[test]
fn summing_input_accepts_single_connection() {
    let registry = DefinitionRegistry::new()
        .with_definition(gain_primitive())
        .with_definition(summing_mixer_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("mix"), "mixer"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("mix"), "inputs"),
        ));

    let validation = definition.validate(&registry);
    assert!(
        validation.is_ok(),
        "summing input with single connection should be valid: {:?}",
        validation.diagnostics()
    );
}

#[test]
fn port_defaults_to_single_source_multiplicity() {
    let port = Port::input("test", SignalType::Audio, 1);
    assert_eq!(port.multiplicity(), Multiplicity::SingleSource);
}

#[test]
fn port_with_summing_multiplicity_reports_summing() {
    let port = Port::input("test", SignalType::Audio, 1).with_multiplicity(Multiplicity::Summing);
    assert_eq!(port.multiplicity(), Multiplicity::Summing);
}

#[test]
fn single_source_error_diagnostic_names_the_port_and_node() {
    let registry = DefinitionRegistry::new().with_definition(gain_primitive());
    let definition = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("a"), "gain"))
        .with_node(Node::new(NodeId::new("b"), "gain"))
        .with_node(Node::new(NodeId::new("c"), "gain"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("a"), "audio_out"),
            PortRef::new(NodeId::new("c"), "audio_in"),
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("b"), "audio_out"),
            PortRef::new(NodeId::new("c"), "audio_in"),
        ));

    let validation = definition.validate(&registry);
    let errors: Vec<_> = validation.diagnostics().errors().collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].module_id(), Some("c"));
    assert_eq!(errors[0].port_name(), Some("audio_in"));
}

// --- 4.1 Poly structural interface ---------------------------------------

#[test]
fn poly_synthesizes_note_forwarded_input_and_wrapped_output_interface() {
    let voice = GraphDefinition::new("voice")
        .with_port(
            Port::input("sidechain", SignalType::Audio, 2).with_multiplicity(Multiplicity::Summing),
        )
        .with_port(
            Port::input("level", SignalType::Control, 1).with_control_default(
                ControlDefault::new(0.75)
                    .with_min(0.0)
                    .with_max(1.0)
                    .with_unit("linear"),
            ),
        )
        .with_port(Port::output("audio", SignalType::Audio, 2))
        .with_port(Port::output("meter", SignalType::Control, 1));
    let registry = poly_registry(voice);
    let root = GraphDefinition::new("root").with_node(poly_node(
        "voices",
        "voice",
        8,
        POLY_ALLOCATION_OLDEST_STEAL,
    ));

    let validation = root.validate(&registry);
    assert!(
        validation.is_ok(),
        "poly interface validates: {validation:?}"
    );
    let ports = root
        .resolved_node_ports(&registry, &NodeId::new("voices"))
        .expect("poly interface resolves");

    assert_eq!(
        ports
            .iter()
            .map(|port| (
                port.name(),
                port.direction(),
                port.signal_type(),
                port.channels(),
                port.multiplicity(),
            ))
            .collect::<Vec<_>>(),
        [
            (
                POLY_NOTE_EVENTS_INPUT,
                PortDirection::Input,
                SignalType::Event,
                1,
                Multiplicity::SingleSource,
            ),
            (
                "sidechain",
                PortDirection::Input,
                SignalType::Audio,
                2,
                Multiplicity::Summing,
            ),
            (
                "level",
                PortDirection::Input,
                SignalType::Control,
                1,
                Multiplicity::SingleSource,
            ),
            (
                "audio",
                PortDirection::Output,
                SignalType::Audio,
                2,
                Multiplicity::SingleSource,
            ),
            (
                "meter",
                PortDirection::Output,
                SignalType::Control,
                1,
                Multiplicity::SingleSource,
            ),
        ]
    );
    let level = ports.iter().find(|port| port.name() == "level").unwrap();
    let default = level.control_default().expect("forwarded default");
    assert_eq!(default.default(), 0.75);
    assert_eq!(default.min(), Some(0.0));
    assert_eq!(default.max(), Some(1.0));
    assert_eq!(default.unit(), Some("linear"));
}

#[test]
fn ordinary_connections_validate_against_the_synthesized_poly_interface() {
    let notes = GraphDefinition::new("notes").with_port(Port::output("out", SignalType::Event, 1));
    let audio = GraphDefinition::new("audio")
        .with_port(Port::output("out", SignalType::Audio, 2))
        .with_port(Port::input("in", SignalType::Audio, 2));
    let voice = GraphDefinition::new("voice")
        .with_port(Port::input("sidechain", SignalType::Audio, 2))
        .with_port(Port::output("audio", SignalType::Audio, 2));
    let registry = poly_registry(voice)
        .with_definition(notes)
        .with_definition(audio);
    let root = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("notes"), "notes"))
        .with_node(Node::new(NodeId::new("source"), "audio"))
        .with_node(poly_node("voices", "voice", 8, POLY_ALLOCATION_REJECT_NEW))
        .with_node(Node::new(NodeId::new("sink"), "audio"))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("notes"), "out"),
            PortRef::new(NodeId::new("voices"), POLY_NOTE_EVENTS_INPUT),
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("source"), "out"),
            PortRef::new(NodeId::new("voices"), "sidechain"),
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("voices"), "audio"),
            PortRef::new(NodeId::new("sink"), "in"),
        ));

    assert!(
        root.validate(&registry).is_ok(),
        "ordinary compatible connections use the synthesized interface"
    );
}

#[test]
fn poly_connections_reject_missing_type_and_channel_mismatches_structurally() {
    let voice = GraphDefinition::new("voice")
        .with_port(Port::input("sidechain", SignalType::Audio, 2))
        .with_port(Port::output("audio", SignalType::Audio, 2));
    let source = GraphDefinition::new("source")
        .with_port(Port::output("event", SignalType::Event, 1))
        .with_port(Port::output("mono", SignalType::Audio, 1));
    let registry = poly_registry(voice).with_definition(source);

    for (port, expected_code) in [
        ("missing", error_codes::KERNEL_MISSING_PORT),
        ("sidechain", error_codes::KERNEL_INCOMPATIBLE_SIGNAL_TYPES),
    ] {
        let root = GraphDefinition::new("root")
            .with_node(Node::new(NodeId::new("source"), "source"))
            .with_node(poly_node(
                "voices",
                "voice",
                8,
                POLY_ALLOCATION_OLDEST_STEAL,
            ))
            .with_connection(Connection::new(
                PortRef::new(NodeId::new("source"), "event"),
                PortRef::new(NodeId::new("voices"), port),
            ));
        assert_eq!(only_error_code(&root.validate(&registry)), expected_code);
    }

    let channel_mismatch = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("source"), "source"))
        .with_node(poly_node(
            "voices",
            "voice",
            8,
            POLY_ALLOCATION_OLDEST_STEAL,
        ))
        .with_connection(Connection::new(
            PortRef::new(NodeId::new("source"), "mono"),
            PortRef::new(NodeId::new("voices"), "sidechain"),
        ));
    assert_eq!(
        only_error_code(&channel_mismatch.validate(&registry)),
        error_codes::KERNEL_CHANNEL_COUNT_MISMATCH
    );
}

#[test]
fn poly_rejects_unconvertible_voice_counts_and_unknown_wrapped_definition() {
    let registry = builtins::builtin_registry();
    for (node, expected_code) in [
        (
            poly_node("zero", "missing", 0, POLY_ALLOCATION_OLDEST_STEAL),
            error_codes::KERNEL_POLY_INVALID_MAX_VOICES,
        ),
        (
            poly_node(
                "wide",
                "missing",
                i64::from(u32::MAX) + 1,
                POLY_ALLOCATION_OLDEST_STEAL,
            ),
            error_codes::KERNEL_POLY_INVALID_MAX_VOICES,
        ),
        (
            poly_node("missing", "does-not-exist", 8, POLY_ALLOCATION_OLDEST_STEAL),
            error_codes::KERNEL_POLY_UNKNOWN_WRAPPED_DEFINITION,
        ),
    ] {
        let validation = GraphDefinition::new("root")
            .with_node(node)
            .validate(&registry);
        assert!(
            validation
                .diagnostics()
                .errors()
                .any(|diagnostic| diagnostic.error_code() == expected_code),
            "expected {expected_code}: {validation:?}"
        );
    }
}

#[test]
fn poly_requires_all_three_structural_static_arguments() {
    let registry = builtins::builtin_registry();
    let validation = GraphDefinition::new("root")
        .with_node(Node::new(NodeId::new("voices"), POLY_DEFINITION))
        .validate(&registry);
    let messages = validation
        .diagnostics()
        .errors()
        .map(|diagnostic| diagnostic.message())
        .collect::<Vec<_>>();

    assert_eq!(messages.len(), 3);
    for parameter in [
        POLY_WRAPPED_DEFINITION_PARAM,
        POLY_MAX_VOICES_PARAM,
        POLY_ALLOCATION_PARAM,
    ] {
        assert!(
            messages.iter().any(|message| message.contains(parameter)),
            "missing required poly argument '{parameter}' is named"
        );
    }
}

#[test]
fn poly_rejects_malformed_wrapped_interfaces() {
    for voice in [
        GraphDefinition::new("voice").with_port(Port::input(
            POLY_NOTE_EVENTS_INPUT,
            SignalType::Event,
            1,
        )),
        GraphDefinition::new("voice").with_port(Port::output(
            "invalid_event",
            SignalType::Event,
            1,
        )),
    ] {
        let registry = poly_registry(voice);
        let validation = GraphDefinition::new("root")
            .with_node(poly_node(
                "voices",
                "voice",
                8,
                POLY_ALLOCATION_OLDEST_STEAL,
            ))
            .validate(&registry);
        assert_eq!(
            only_error_code(&validation),
            error_codes::KERNEL_POLY_MALFORMED_INTERFACE
        );
    }
}

#[test]
fn poly_accepts_designated_done_output_without_exposing_it_publicly() {
    let voice = GraphDefinition::new("voice")
        .with_port(Port::output("audio", SignalType::Audio, 1))
        .with_port(Port::output(POLY_DONE_OUTPUT, SignalType::Event, 1));
    let registry = poly_registry(voice);
    let root = GraphDefinition::new("root").with_node(poly_node(
        "voices",
        "voice",
        8,
        POLY_ALLOCATION_OLDEST_STEAL,
    ));

    let validation = root.validate(&registry);
    assert!(validation.is_ok(), "done is a valid lifecycle output");
    let ports = root
        .resolved_node_ports(&registry, &NodeId::new("voices"))
        .expect("poly interface resolves");
    assert!(ports.iter().any(|port| port.name() == "audio"));
    assert!(ports.iter().all(|port| port.name() != POLY_DONE_OUTPUT));
}

#[test]
fn nested_poly_interface_validation_reaches_the_inner_poly() {
    let inner = GraphDefinition::new("inner").with_port(Port::output(
        "invalid_event",
        SignalType::Event,
        1,
    ));
    let outer = GraphDefinition::new("outer").with_node(poly_node(
        "inner_voices",
        "inner",
        2,
        POLY_ALLOCATION_REJECT_NEW,
    ));
    let registry = builtins::builtin_registry()
        .with_definition(inner)
        .with_definition(outer);
    let validation = GraphDefinition::new("root")
        .with_node(poly_node(
            "outer_voices",
            "outer",
            2,
            POLY_ALLOCATION_REJECT_NEW,
        ))
        .validate(&registry);

    assert!(
        validation.diagnostics().errors().any(|diagnostic| {
            diagnostic.error_code() == error_codes::KERNEL_POLY_MALFORMED_INTERFACE
                && diagnostic.module_id() == Some("inner_voices")
        }),
        "nested malformed poly is diagnosed: {validation:?}"
    );
}
