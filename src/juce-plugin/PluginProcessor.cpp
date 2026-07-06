#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "DefaultPatch.h"

DandrumAudioProcessor::DandrumAudioProcessor()
    : juce::AudioProcessor (BusesProperties()
                                 .withInput ("Input", juce::AudioChannelSet::stereo(), true)
                                 .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      engine (dandrum_engine_create())
{
    instrumentLoaded = loadDefaultInstrument();
}

DandrumAudioProcessor::~DandrumAudioProcessor()
{
    dandrum_engine_destroy (engine);
}

bool DandrumAudioProcessor::loadDefaultInstrument()
{
    if (engine == nullptr)
    {
        lastLoadError = "Rust engine was not created";
        return false;
    }

    const auto patchPath = dandrum::defaultPatchPath();
    if (! dandrum_engine_load_patch (engine, patchPath.string().c_str()))
    {
        lastLoadError = "Failed to load default patch: " + patchPath.string();
        return false;
    }

    lastLoadError = {};
    return true;
}

void DandrumAudioProcessor::prepareToPlay (double sampleRate, int samplesPerBlock)
{
    if (engine == nullptr || ! instrumentLoaded)
        return;

    dandrum_engine_prepare_realtime (engine,
                                     static_cast<float> (sampleRate),
                                     static_cast<std::size_t> (juce::jmax (1, samplesPerBlock)));
}

void DandrumAudioProcessor::releaseResources() {}

bool DandrumAudioProcessor::isBusesLayoutSupported (const BusesLayout& layouts) const
{
    if (layouts.getMainOutputChannelSet() != juce::AudioChannelSet::stereo())
        return false;

    const auto input = layouts.getMainInputChannelSet();
    return input == juce::AudioChannelSet::disabled() || input == juce::AudioChannelSet::stereo();
}

void DandrumAudioProcessor::renderSilence (juce::AudioBuffer<float>& buffer) const
{
    buffer.clear();
}

void DandrumAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ScopedNoDenormals noDenormals;

    const auto numSamples = buffer.getNumSamples();

    for (auto channel = 2; channel < buffer.getNumChannels(); ++channel)
        buffer.clear (channel, 0, numSamples);

    if (engine == nullptr || ! instrumentLoaded || numSamples <= 0 || buffer.getNumChannels() <= 0)
    {
        renderSilence (buffer);
        return;
    }

    for (const auto metadata : midiMessages)
    {
        const auto message = metadata.getMessage();
        const auto frameOffset = static_cast<std::size_t> (juce::jlimit (0, numSamples - 1, metadata.samplePosition));

        if (message.isNoteOn())
        {
            dandrum_engine_note_on_at (engine,
                                       static_cast<unsigned char> (message.getNoteNumber()),
                                       static_cast<unsigned char> (message.getVelocity() * 127.0f),
                                       frameOffset);
        }
        else if (message.isNoteOff())
        {
            dandrum_engine_note_off_at (engine,
                                        static_cast<unsigned char> (message.getNoteNumber()),
                                        frameOffset);
        }
    }

    auto* left = buffer.getWritePointer (0);
    auto* right = buffer.getNumChannels() > 1 ? buffer.getWritePointer (1) : left;

    dandrum_engine_render (engine, left, right, static_cast<std::size_t> (numSamples));
}

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumAudioProcessorEditor (*this);
}

bool DandrumAudioProcessor::hasEditor() const
{
    return true;
}

const juce::String DandrumAudioProcessor::getName() const
{
    return JucePlugin_Name;
}

bool DandrumAudioProcessor::acceptsMidi() const
{
    return true;
}

bool DandrumAudioProcessor::producesMidi() const
{
    return false;
}

double DandrumAudioProcessor::getTailLengthSeconds() const
{
    return 0.0;
}

int DandrumAudioProcessor::getNumPrograms()
{
    return 1;
}

int DandrumAudioProcessor::getCurrentProgram()
{
    return 0;
}

void DandrumAudioProcessor::setCurrentProgram (int) {}

const juce::String DandrumAudioProcessor::getProgramName (int)
{
    return {};
}

void DandrumAudioProcessor::changeProgramName (int, const juce::String&) {}

void DandrumAudioProcessor::getStateInformation (juce::MemoryBlock&) {}

void DandrumAudioProcessor::setStateInformation (const void*, int) {}

bool DandrumAudioProcessor::isInstrumentLoaded() const noexcept
{
    return instrumentLoaded;
}

const juce::String& DandrumAudioProcessor::getLastLoadError() const noexcept
{
    return lastLoadError;
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new DandrumAudioProcessor();
}
