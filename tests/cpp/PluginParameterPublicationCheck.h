#pragma once

// Fast contract through the original processor/editor and registered commands.
namespace parameterPublicationCheck
{
template <typename Probe>
int run()
{
    try
    {
        auto configuration = InstrumentDemoConfiguration::sampler();
        configuration.instrumentPath = juce::File (DANDRUM_SOURCE_ROOT)
            .getChildFile ("tests/fixtures/plugin-ui-knob.yaml").getFullPathName().toStdString();
        DandrumAudioProcessor processor (configuration);
        processor.setPlayConfigDetails (0, 2, 48000.0, 64);
        processor.prepareToPlay (48000.0, 64);
        DandrumAudioProcessorEditor editor (processor);
        editor.addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        editor.setVisible (true);
        const auto check = [] (bool valid, const char* message)
        {
            if (!valid) throw std::runtime_error (message);
        };
        const auto state = [&] { return Probe::invoke (editor, "getParameterState"); };
        const auto publish = [&] { Probe::publishUnchangedSurface (editor); };
        const auto ack = [&] (juce::var ticket, juce::var generation)
        {
            return static_cast<bool> (Probe::invoke (editor, "ackParameterState", { ticket, generation }));
        };
        auto* parameter = processor.getParameterForPublicId ("fixture.level");
        check (parameter != nullptr, "Parameter publication fixture did not expose level");
        const auto generation = static_cast<int> (processor.getParameterSurfaceGeneration());
        const auto hostCount = processor.getParameters().size();
        check (state()["publication"].isVoid(), "State query started a timer publication");
        publish();
        const auto first = state()["publication"];
        check (first.isString() && first.toString().isNotEmpty(),
               "Parameter timer has no bounded publication identity");
        for (int tick = 0; tick < 128; ++tick)
        {
            parameter->setValueNotifyingHost (tick == 127 ? 0.75f : 0.25f);
            publish();
            check (state()["publication"] == first,
                   "Stalled parameter consumer admitted another timer publication");
        }
        check (!ack ("bad", generation) && !ack ("0", generation)
            && !ack (1.0, generation) && !ack ("18446744073709551616", generation)
            && !ack (first, generation + 1) && !ack (first, -1)
            && !ack (first, 0.5) && !ack (first, "1"),
            "Malformed or stale parameter acknowledgement released publication");
        DandrumAudioProcessorEditor other (processor);
        other.addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        other.setVisible (true);
        Probe::publishUnchangedSurface (other);
        const auto otherTicket = Probe::invoke (other, "getParameterState")["publication"];
        check (otherTicket.isString() && otherTicket != first,
               "Two editors reused the same parameter publication identity");
        check (!static_cast<bool> (Probe::invoke (other, "ackParameterState", { first, generation })),
               "Another editor acknowledged a publication it does not own");
        check (static_cast<bool> (Probe::invoke (other, "ackParameterState", { otherTicket, generation })),
               "Second editor could not acknowledge its own publication");
        check (ack (first, generation) && !ack (first, generation),
               "Exact parameter acknowledgement did not release exactly once");
        check (state()["publication"].isVoid(), "Acknowledged publication remained outstanding");
        publish();
        const auto latest = state();
        const auto second = latest["publication"];
        check (second.isString() && second != first
            && std::abs (static_cast<double> (latest["parameters"].getArray()->getReference (0)["value"]) - 0.75) < 1e-6,
            "Acknowledgement did not publish current host values with a fresh identity");
        check (!ack (first, generation), "Late acknowledgement released a newer publication");
        check (processor.getParameterForPublicId ("fixture.level") == parameter
            && processor.getParameters().size() == hostCount
            && processor.getParameterSurfaceGeneration() == static_cast<std::uint32_t> (generation),
            "Bounded UI publication replaced ordinary host parameter identities");
        juce::AudioBuffer<float> audio (2, 64);
        juce::MidiBuffer midi;
        processor.processBlock (audio, midi);
        for (int frame = 0; frame < 64; ++frame)
            check (std::bit_cast<std::uint32_t> (audio.getSample (0, frame)) == 0x3f000000U
                && std::bit_cast<std::uint32_t> (audio.getSample (1, frame)) == 0U,
                   "Stalled host-state publication changed signed live audio");
        check (ack (second, generation), "Could not release publication before hiding");
        editor.setVisible (false);
        for (int tick = 0; tick < 128; ++tick) publish();
        check (state()["publication"].isVoid(), "Hidden browser started parameter publications");
        editor.setVisible (true); publish();
        const auto hidden = state()["publication"];
        check (hidden.isString() && hidden != second, "Shown browser did not resume parameter publication");
        check (processor.reloadInstrumentFromFile (juce::File (juce::String (configuration.instrumentPath))),
               "Could not reload parameter publication fixture");
        check (!ack (hidden, generation), "Retired generation acknowledged current parameter state");
        check (state()["publication"].isVoid()
            && !ack (hidden, static_cast<int> (processor.getParameterSurfaceGeneration())),
            "Current-generation query or acknowledgement reused a retired document ticket");
        publish(); // The original bridge refreshes the document on generation change.
        check (state()["publication"].isVoid(), "Reload retained the old document publication");
        publish();
        const auto replacement = state();
        check (replacement["publication"].isString() && replacement["publication"] != hidden
            && static_cast<int> (replacement["generation"]) > generation
            && !ack (hidden, replacement["generation"])
            && ack (replacement["publication"], replacement["generation"]),
            "Reload did not require a fresh owned parameter acknowledgement");
        check (!static_cast<bool> (Probe::invoke (editor, "ackParameterState")),
               "Missing parameter acknowledgement arguments were accepted");
        std::cout << "PARAMETER_PUBLICATION ticks=256 current_signed_audio=0.5 hidden_reload_sessions=PASS\n";
        return 0;
    }
    catch (const std::exception& error)
    {
        std::cerr << "Parameter publication failed: " << error.what() << '\n';
        return 1;
    }
}
}
