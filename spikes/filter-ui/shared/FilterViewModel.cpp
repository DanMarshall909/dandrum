#include "FilterViewModel.h"
#include "FilterKernel.h"
namespace filter_spike {
FilterViewModel::FilterViewModel(FilterProcessor& source) : processor(source) {
    visual.inputDb.fill(-90);visual.outputDb.fill(-90);visual.historyDb.fill(-90);
    for(size_t i=0;i<window.size();++i)
        window[i]=.5f-.5f*float(std::cos(2*juce::MathConstants<double>::pi*double(i)/double(fftSize-1)));
    previous.fill(std::numeric_limits<float>::quiet_NaN());update();
}
FilterViewModel::~FilterViewModel() { endKnob();endNode();if(responseEngine) dandrum_kernel_destroy(responseEngine); }
void FilterViewModel::selectBand(int band) { endKnob();endNode();selected=juce::jlimit(0,2,band); }
int FilterViewModel::parameterForKnob(int knob) const {
    const int frequencies[]{hpFrequency,bellFrequency,lpFrequency};
    const int qualities[]{hpQ,bellQ,lpQ};
    return knob==0 ? frequencies[selected] : knob==1 ? qualities[selected] : bellGain;
}
float FilterViewModel::knobValue(int knob) const { return processor.normalized(parameterForKnob(knob)); }
juce::String FilterViewModel::knobLabel(int knob) const { return knob==0 ? "FREQUENCY" : knob==1 ? "RESONANCE Q" : "BELL GAIN"; }
juce::String FilterViewModel::knobText(int knob) const {
    const float value=processor.actual(parameterForKnob(knob));
    if(knob==0) return value>=1000 ? juce::String(value/1000,2)+" kHz" : juce::String(value,0)+" Hz";
    return juce::String(value,knob==1 ? 2 : 1)+(knob==2 ? " dB" : "");
}
void FilterViewModel::beginKnob(int knob) {
    endKnob();endNode();activeKnob=knob;activeKnobParameter=parameterForKnob(knob);
    processor.beginGesture(activeKnobParameter);
}
void FilterViewModel::setKnob(int knob,float value) {
    processor.setNormalized(activeKnob==knob ? activeKnobParameter : parameterForKnob(knob),value);
}
void FilterViewModel::endKnob() {
    if(activeKnob>=0) processor.endGesture(activeKnobParameter);
    activeKnob=-1;activeKnobParameter=-1;
}
void FilterViewModel::beginNode(int band) {
    selectBand(band);activeNode=selected;
    processor.beginGesture(parameterForKnob(0));
    processor.beginGesture(selected==1 ? bellGain : parameterForKnob(1));
}
void FilterViewModel::dragNode(float x,float y) {
    if(activeNode<0) return;
    const int frequency=parameterForKnob(0),vertical=selected==1 ? bellGain : parameterForKnob(1);
    const float db=maximumDb-juce::jlimit(0.0f,1.0f,y)*(maximumDb-minimumDb);
    processor.setNormalized(frequency,processor.normalize(frequency,frequencyAt(juce::jlimit(0.0f,1.0f,x))));
    processor.setNormalized(vertical,processor.normalize(vertical,selected==1 ? db : std::pow(10.0f,db/20)));
}
void FilterViewModel::endNode() {
    if(activeNode>=0) { processor.endGesture(parameterForKnob(0));
        processor.endGesture(selected==1 ? bellGain : parameterForKnob(1)); }
    activeNode=-1;
}
void FilterViewModel::toggle(int parameter) {
    processor.beginGesture(parameter);processor.setNormalized(parameter,enabled(parameter) ? 0 : 1);processor.endGesture(parameter);
}
bool FilterViewModel::update() {
    std::array<float,7> current{};
    for(size_t i=0;i<current.size();++i) current[i]=processor.actual(int(i));
    if(current!=previous || previousRate!=processor.analysisSampleRate()) { previous=current;updateResponse(); }
    const int frequencies[]{hpFrequency,bellFrequency,lpFrequency},qualities[]{hpQ,bellQ,lpQ};
    for(int i=0;i<3;++i) visual.nodes[size_t(i)]={frequencyX(current[size_t(frequencies[i])]),
        responseY(i==1 ? current[bellGain] : 20*std::log10(current[size_t(qualities[i])]))};
    const auto levels=processor.levels();
    for(size_t i=0;i<levels.size();++i) visual.meters[i]=std::max(levels[i],visual.meters[i]*.82f);
    bool changed=false;
    // Bounded catch-up allows the UI to recover after a delayed frame without blocking audio.
    for(int count=0;count<4 && processor.consumeCapture(input,output);++count) { updateSpectrum();changed=true; }
    ++visual.sequence;return changed;
}
}
