#include "FilterViewModel.h"
#include <iostream>
#include <stdexcept>
namespace filter_spike { juce::AudioProcessorEditor* makeFilterEditor(FilterProcessor&) { return nullptr; } }
void require(bool value, const char* message) { if(!value) throw std::runtime_error(message); }
struct Listener : juce::AudioProcessorListener {
    int begins=0,ends=0,values=0;
    void audioProcessorParameterChanged(juce::AudioProcessor*,int,float) override { ++values; }
    void audioProcessorChanged(juce::AudioProcessor*,const ChangeDetails&) override {}
    void audioProcessorParameterChangeGestureBegin(juce::AudioProcessor*,int) override { ++begins; }
    void audioProcessorParameterChangeGestureEnd(juce::AudioProcessor*,int) override { ++ends; }
};
int main() {
 try {
    juce::ScopedJuceInitialiser_GUI gui;
    filter_spike::FilterProcessor first, restored;
    first.prepareToPlay(48000,64); restored.prepareToPlay(48000,64);
    Listener listener; first.addListener(&listener);
    {
      filter_spike::FilterViewModel model(first);
      model.beginNode(1); model.dragNode(.65f,.22f); model.endNode();
      require(first.actual(filter_spike::bellGain)>5,"Graph drag must update host gain");
      model.beginKnob(0); model.setKnob(0,.55f); model.endKnob();
      require(listener.begins==3 && listener.ends==3 && listener.values>=3,"Balanced host gestures");
      model.beginKnob(0); // Closing an editor mid-gesture must still notify the host.
    }
    require(listener.begins==4 && listener.ends==4,"Closing model ends active gesture");
    juce::MemoryBlock state; first.getStateInformation(state);
    restored.setStateInformation(state.getData(),int(state.getSize()));
    juce::MidiBuffer midi; juce::AudioBuffer<float> a(2,64), b(2,64);
    a.clear();b.clear();a.setSample(0,0,.5f);a.setSample(1,0,-.25f);b.makeCopyOf(a);
    first.processBlock(a,midi);restored.processBlock(b,midi);
    for(int ch=0;ch<2;++ch) for(int i=0;i<64;++i)
      require(std::abs(a.getSample(ch,i)-b.getSample(ch,i))<1e-7,"Restored state must render identically");
    first.setNormalized(filter_spike::bypass,1);
    a.clear();a.setSample(0,0,.5f);a.setSample(1,0,-.25f);first.processBlock(a,midi);
    require(a.getSample(0,0)==.5f && a.getSample(1,0)==-.25f && a.getSample(0,1)==0,"Bypass exact signed stereo");
    a.setSize(2,129);a.clear();a.setSample(0,0,.5f);a.setSample(1,0,-.25f);
    first.processBlock(a,midi);
    require(first.levels()[0]==.5f && first.levels()[2]==.5f,"Meters retain peak across oversized callback chunks");
    a.setSize(2,64);
    filter_spike::FilterViewModel analysis(first);
    for(int block=0;block<48;++block) {
      for(int i=0;i<64;++i) for(int ch=0;ch<2;++ch)
        a.setSample(ch,i,.25f*std::sin(2*juce::MathConstants<double>::pi*1500*(block*64+i)/48000));
      first.processBlock(a,midi);
    }
    analysis.update();
    const auto& frame=analysis.frame();
    require(frame.historyHead>0 && frame.sequence>0,"Actual capture advances history");
    require(*std::max_element(frame.inputDb.begin(),frame.inputDb.end())>-25,"Known sine appears in spectrum");
    require(frame.meters[0]>.2f && frame.meters[2]>.2f,"Actual input/output meters");
    first.setNormalized(filter_spike::bellFrequency,first.normalize(filter_spike::bellFrequency,1000));
    first.setNormalized(filter_spike::bellGain,first.normalize(filter_spike::bellGain,12));
    analysis.update();
    const int centre=int(std::round(filter_spike::frequencyX(1000)*(filter_spike::plotBins-1)));
    require(std::abs(analysis.frame().responseDb[size_t(centre)]-12)<.3f,"Actual Rust response shows known bell boost");
    first.setNormalized(filter_spike::audition,1);a.clear();first.processBlock(a,midi);
    require(a.getMagnitude(0,0,64)>.02f && a.getMagnitude(0,0,64)<.17f,"Audition generates a quiet signal without input");
    for(int i=0;i<64;++i) require(a.getSample(0,i)==a.getSample(1,i),"Audition feeds both stereo input channels");
    require(restored.actual(filter_spike::bypass)==0,"Independent processor state");
    filter_spike::FilterProcessor spectral; spectral.prepareToPlay(48000,64);
    spectral.setNormalized(filter_spike::lpFrequency,spectral.normalize(filter_spike::lpFrequency,500));
    filter_spike::FilterViewModel filteredAnalysis(spectral);
    for(int block=0;block<96;++block) {
      for(int i=0;i<64;++i) for(int ch=0;ch<2;++ch)
        a.setSample(ch,i,.25f*std::sin(2*juce::MathConstants<double>::pi*1500*(block*64+i)/48000));
      spectral.processBlock(a,midi);
      if(block%32==31) filteredAnalysis.update();
    }
    const auto& filtered=filteredAnalysis.frame();
    const auto peak=size_t(std::max_element(filtered.inputDb.begin(),filtered.inputDb.end())-filtered.inputDb.begin());
    require(std::abs(filter_spike::frequencyAt(float(peak)/(filter_spike::plotBins-1))-1500)<150,"Spectrum peak matches the known input frequency");
    require(filtered.inputDb[peak]-filtered.outputDb[peak]>15,"Output spectrum shows low-pass attenuation of known input");
    const auto historyRow=(filtered.historyHead+filter_spike::historyColumns-1)%filter_spike::historyColumns;
    require(filtered.historyDb[size_t(historyRow*filter_spike::plotBins)+peak]==filtered.outputDb[peak],"History records observed filtered output");
    first.removeListener(&listener);
    std::cout << "State, bypass, gestures, captured spectrum/history/meters and independent state passed\n";
 } catch(const std::exception& e) { std::cerr<<e.what()<<'\n';return 1; }
}
