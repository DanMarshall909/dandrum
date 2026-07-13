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
