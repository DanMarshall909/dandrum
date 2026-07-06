#include "PluginEditor.h"

DandrumAudioProcessorEditor::DandrumAudioProcessorEditor (DandrumAudioProcessor& processorToUse)
    : juce::AudioProcessorEditor (&processorToUse),
      processor (processorToUse)
{
    const auto statusText = processor.isInstrumentLoaded()
                                ? juce::String ("Dandrum")
                                : juce::String ("Dandrum: ") + processor.getLastLoadError();
    statusLabel.setText (statusText, juce::dontSendNotification);
    statusLabel.setJustificationType (juce::Justification::centredLeft);
    addAndMakeVisible (statusLabel);

    for (auto* parameter : processor.getParameters())
    {
        auto* rangedParameter = dynamic_cast<juce::RangedAudioParameter*> (parameter);
        if (rangedParameter == nullptr)
            continue;

        ParameterControl control;
        control.parameter = rangedParameter;
        control.label = std::make_unique<juce::Label>();
        control.slider = std::make_unique<juce::Slider>();

        control.label->setText (rangedParameter->getName (64), juce::dontSendNotification);
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

    setSize (juce::jmax (420, 160 * juce::jmax (1, static_cast<int> (parameterControls.size()))), 320);
}

DandrumAudioProcessorEditor::~DandrumAudioProcessorEditor() = default;

void DandrumAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (getLookAndFeel().findColour (juce::ResizableWindow::backgroundColourId));
}

void DandrumAudioProcessorEditor::resized()
{
    auto area = getLocalBounds().reduced (12);
    statusLabel.setBounds (area.removeFromTop (28));
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
