#include "PluginEditor.h"

DandrumAudioProcessorEditor::DandrumAudioProcessorEditor (DandrumAudioProcessor& processorToUse)
    : juce::AudioProcessorEditor (&processorToUse),
      processor (processorToUse)
{
    statusLabel.setJustificationType (juce::Justification::centredLeft);
    addAndMakeVisible (statusLabel);

    loadPresetButton.onClick = [this]
    {
        presetChooser = std::make_unique<juce::FileChooser> (
            "Load Dandrum preset",
            juce::File(),
            "*.yaml;*.yml");

        presetChooser->launchAsync (
            juce::FileBrowserComponent::openMode | juce::FileBrowserComponent::canSelectFiles,
            [this] (const juce::FileChooser& chooser)
            {
                const auto file = chooser.getResult();
                if (file.existsAsFile())
                    processor.loadPresetFromFile (file);

                updateStatusLabel();
            });
    };
    addAndMakeVisible (loadPresetButton);

    rebuildControls();
    updateStatusLabel();
    startTimerHz (8);

    setSize (juce::jmax (420, 160 * juce::jmax (1, static_cast<int> (parameterControls.size()))), 360);
}

DandrumAudioProcessorEditor::~DandrumAudioProcessorEditor() = default;

void DandrumAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (getLookAndFeel().findColour (juce::ResizableWindow::backgroundColourId));
}

void DandrumAudioProcessorEditor::resized()
{
    auto area = getLocalBounds().reduced (12);
    auto top = area.removeFromTop (30);
    statusLabel.setBounds (top.removeFromLeft (juce::jmax (120, top.getWidth() - 150)));
    loadPresetButton.setBounds (top.removeFromRight (140));
    area.removeFromTop (12);

    const auto controlWidth = 120;
    const auto labelHeight = 22;
    const auto controlHeight = 140;
    auto x = area.getX();
    auto y = area.getY();

    for (auto& control : parameterControls)
    {
        if (x + controlWidth > area.getRight())
        {
            x = area.getX();
            y += labelHeight + controlHeight + 12;
        }

        control.label->setBounds (x, y, controlWidth, labelHeight);
        control.slider->setBounds (x, y + labelHeight, controlWidth, controlHeight);
        x += controlWidth + 12;
    }
}

void DandrumAudioProcessorEditor::rebuildControlsIfNeeded()
{
    if (lastSeenParameterSurfaceGeneration != processor.getParameterSurfaceGeneration())
        rebuildControls();
}

void DandrumAudioProcessorEditor::rebuildControls()
{
    for (auto& control : parameterControls)
    {
        if (control.label != nullptr)
            removeChildComponent (control.label.get());
        if (control.slider != nullptr)
            removeChildComponent (control.slider.get());
    }
    parameterControls.clear();

    for (const auto& publicId : processor.getActivePublicParameterIds())
    {
        auto* rangedParameter = processor.getParameterForPublicId (publicId);
        if (rangedParameter == nullptr)
            continue;

        ParameterControl control;
        control.publicId = publicId;
        control.parameter = rangedParameter;
        control.label = std::make_unique<juce::Label>();
        control.slider = std::make_unique<juce::Slider>();

        auto labelText = processor.getPublicParameterDisplayName (publicId);
        if (labelText.isEmpty())
            labelText = publicId;

        control.label->setText (labelText, juce::dontSendNotification);
        control.label->setJustificationType (juce::Justification::centredLeft);
        addAndMakeVisible (*control.label);

        control.slider->setSliderStyle (juce::Slider::RotaryHorizontalVerticalDrag);
        control.slider->setTextBoxStyle (juce::Slider::TextBoxBelow, false, 80, 20);
        control.slider->setRange (0.0, 1.0, 0.001);
        control.slider->setValue (rangedParameter->getValue(), juce::dontSendNotification);
        control.slider->onValueChange = [slider = control.slider.get(), rangedParameter]
        {
            rangedParameter->beginChangeGesture();
            rangedParameter->setValueNotifyingHost (static_cast<float> (slider->getValue()));
            rangedParameter->endChangeGesture();
        };
        addAndMakeVisible (*control.slider);

        parameterControls.push_back (std::move (control));
    }

    lastSeenParameterSurfaceGeneration = processor.getParameterSurfaceGeneration();
    resized();
}

void DandrumAudioProcessorEditor::updateStatusLabel()
{
    juce::String statusText;
    if (! processor.isInstrumentLoaded())
    {
        statusText = juce::String ("Dandrum: ") + processor.getLastLoadError();
    }
    else
    {
        statusText = juce::String ("Dandrum - ") + processor.replacementTransactionState();
        if (processor.currentPresetName().isNotEmpty())
            statusText += " - Preset: " + processor.currentPresetName();
        if (processor.getLastReloadWarning().isNotEmpty())
            statusText += " - Warning: " + processor.getLastReloadWarning();
        if (processor.getLastPresetError().isNotEmpty())
            statusText += " - Preset error: " + processor.getLastPresetError();
        if (processor.getDroppedMidiEventCount() > 0)
            statusText += " - Dropped MIDI events: " + juce::String (static_cast<int> (processor.getDroppedMidiEventCount()));
    }

    statusLabel.setText (statusText, juce::dontSendNotification);
}

void DandrumAudioProcessorEditor::timerCallback()
{
    rebuildControlsIfNeeded();
    updateStatusLabel();

    for (auto& control : parameterControls)
    {
        if (control.parameter != nullptr && control.slider != nullptr && ! control.slider->isMouseButtonDown())
            control.slider->setValue (control.parameter->getValue(), juce::dontSendNotification);
    }
}
