#include "FilterProcessor.h"
namespace filter_spike {
juce::AudioProcessorValueTreeState::ParameterLayout FilterProcessor::layout() {
    juce::AudioProcessorValueTreeState::ParameterLayout result;
    for(int i=0;i<7;++i) {
        const bool frequency=i==hpFrequency || i==bellFrequency || i==lpFrequency;
        auto range=frequency ? juce::NormalisableRange<float>(20,20000) :
            i==bellGain ? juce::NormalisableRange<float>(-24,24) : juce::NormalisableRange<float>(.1f,10);
        if(frequency) range.setSkewForCentre(1000);
        const float initial=i==hpFrequency ? 100 : i==bellFrequency ? 1000 : i==lpFrequency ? 8000 :
                            i==bellGain ? 0 : std::sqrt(.5f);
        result.add(std::make_unique<juce::AudioParameterFloat>(juce::ParameterID(parameterIds[size_t(i)],1),
            juce::String(parameterIds[size_t(i)]).replaceCharacter('_',' '),range,initial));
    }
    result.add(std::make_unique<juce::AudioParameterBool>(juce::ParameterID("bypass",1),"Bypass",false));
    result.add(std::make_unique<juce::AudioParameterBool>(juce::ParameterID("audition",1),"Audition signal",false));
    return result;
}
FilterProcessor::FilterProcessor() : AudioProcessor(BusesProperties()
    .withInput("Input",juce::AudioChannelSet::stereo(),true)
    .withOutput("Output",juce::AudioChannelSet::stereo(),true)), parameters(*this,nullptr,"FilterLab",layout()) {}
FilterProcessor::~FilterProcessor() { releaseResources(); }
bool FilterProcessor::isBusesLayoutSupported(const BusesLayout& buses) const {
    return buses.getMainInputChannelSet()==juce::AudioChannelSet::stereo() &&
           buses.getMainOutputChannelSet()==juce::AudioChannelSet::stereo();
}
juce::AudioProcessorEditor* FilterProcessor::createEditor() { return makeFilterEditor(*this); }
float FilterProcessor::actual(int i) const { return parameters.getRawParameterValue(parameterIds.at(size_t(i)))->load(); }
float FilterProcessor::normalized(int i) const { return parameters.getParameter(parameterIds.at(size_t(i)))->getValue(); }
float FilterProcessor::normalize(int i,float value) const {
    return parameters.getParameter(parameterIds.at(size_t(i)))->convertTo0to1(value);
}
void FilterProcessor::beginGesture(int i) { parameters.getParameter(parameterIds.at(size_t(i)))->beginChangeGesture(); }
void FilterProcessor::setNormalized(int i,float value) {
    parameters.getParameter(parameterIds.at(size_t(i)))->setValueNotifyingHost(juce::jlimit(0.0f,1.0f,value));
}
void FilterProcessor::endGesture(int i) { parameters.getParameter(parameterIds.at(size_t(i)))->endChangeGesture(); }
void FilterProcessor::getStateInformation(juce::MemoryBlock& destination) {
    if(auto xml=parameters.copyState().createXml()) copyXmlToBinary(*xml,destination);
}
void FilterProcessor::setStateInformation(const void* data,int size) {
    if(auto xml=getXmlFromBinary(data,size); xml && xml->hasTagName(parameters.state.getType()))
        parameters.replaceState(juce::ValueTree::fromXml(*xml));
}
std::array<float,4> FilterProcessor::levels() const {
    std::array<float,4> result{};
    for(size_t i=0;i<result.size();++i) result[i]=peaks[i].load();
    return result;
}
bool FilterProcessor::consumeCapture(std::span<float> input,std::span<float> output) {
    const int count=int(std::min(input.size(),output.size()));
    if(fifo.getNumReady()<count) return false;
    int a,n,b,m; fifo.prepareToRead(count,a,n,b,m);
    for(int i=0;i<n+m;++i) { const auto slot=size_t(i<n ? a+i : b+i-n);
        input[size_t(i)]=captureInput[slot];output[size_t(i)]=captureOutput[slot]; }
    fifo.finishedRead(n+m);return true;
}
}
