#include "PluginProcessor.h"
#include "PluginEditor.h"
#include <algorithm>
#include <chrono>
#include <csignal>
#include <iostream>
#include <memory>
#include <vector>

struct PluginEditorBridgeTestProbe {
    static juce::WebBrowserComponent& browser(DandrumAudioProcessorEditor& editor) { return editor.browser; }
};
static double now() { return juce::Time::getMillisecondCounterHiRes(); }
static double quantile(std::vector<double> values, double q) {
    std::sort(values.begin(), values.end()); return values.at(static_cast<size_t>(q * (values.size() - 1)));
}
static void audioProbe(const InstrumentDemoConfiguration& config) {
    for (int size : {64, 128, 256, 512}) {
        DandrumAudioProcessor processor(config);
        processor.setPlayConfigDetails(0, 2, 48000, size);
        processor.prepareToPlay(48000, size);
        if (!processor.isInstrumentLoaded()) throw std::runtime_error(processor.getLastLoadError().toStdString());
        const int note = config.instrumentId == "dandrum.tb303-acid" ? 60 : 36;
        juce::AudioBuffer<float> buffer(2, size);
        juce::MidiBuffer midi;
        midi.addEvent(juce::MidiMessage::noteOn(1, static_cast<juce::uint8>(note), 0.9f), 20);
        processor.processBlock(buffer, midi);
        int first = -1;
        for (int i = 0; i < size; ++i) if (std::abs(buffer.getSample(0,i)) > 1e-7f) { first = i; break; }
        std::vector<double> times;
        for (int i = 0; i < 2032; ++i) {
            midi.clear();
            if (i % 100 == 0) midi.addEvent(juce::MidiMessage::noteOn(1, static_cast<juce::uint8>(note), 0.9f), 0);
            if (i % 100 == 50) midi.addEvent(juce::MidiMessage::noteOff(1, note), 0);
            const double start = now(); processor.processBlock(buffer, midi);
            if (i >= 32) times.push_back(now() - start);
        }
        std::cout << "AUDIO " << config.instrumentId << " size=" << size << " host_latency_samples=" << processor.getLatencySamples()
                  << " note_offset=20 first_nonzero=" << first << " render_ms_p50=" << quantile(times,.5)
                  << " p95=" << quantile(times,.95) << " p99=" << quantile(times,.99) << " max=" << quantile(times,1) << std::endl;
    }
}
static const char* script = R"JS(
(() => {
  if (window.__latencyProbe) return;
  const report = window.__latencyProbe = {stage:'waiting'};
  const pause = ms => new Promise(r => setTimeout(r,ms));
  const stats = a => {a.sort((x,y)=>x-y);return {n:a.length,p50:a[Math.floor((a.length-1)*.5)],p95:a[Math.floor((a.length-1)*.95)],p99:a[Math.floor((a.length-1)*.99)],max:a.at(-1)}};
  const run = async () => {
    try {
      const deadline = performance.now()+20000;
      let knob;
      while (!(knob = document.querySelector('[data-parameter-id][aria-disabled="false"]'))) {
        if (performance.now()>deadline) throw Error('React controls failed to load'); await pause(10);
      }
      await pause(500);
      if(window.__latencyUnscale) {
        const style=document.createElement('style');style.textContent='.machine { transform: none !important; }';document.head.append(style);await pause(500);
      }
      report.stage='running';
      const backend=window.__JUCE__.backend;
      const base=backend.getNativeFunction;
      const checked=reply=>{if(typeof reply==='string'||(reply?.status&&reply.status!=='accepted'))throw Error('Rejected native command: '+JSON.stringify(reply));return reply};
      const invoke=async(name,...args)=>checked(await base(name)(...args));
      const state=await invoke('getParameterState');
      const id=knob.dataset.parameterId, generation=state.generation;
      const original=state.parameters.find(p=>p.id===id).value;
      report.parameter=id;
      const roundtrip=[], writeRead=[], note=[];
      for (let i=0;i<100;++i) {const t=performance.now();await invoke('getParameterState');roundtrip.push(performance.now()-t)}
      await invoke('beginGesture',id,generation);
      for (let i=0;i<100;++i) {const t=performance.now();await invoke('setParameter',id,i%2?.51:.5,generation);await invoke('getParameterState');writeRead.push(performance.now()-t)}
      await invoke('endGesture',id,generation);
      const pitch = document.querySelector('.tb303') ? 60 : (document.title.includes('303') ? 60 : 36);
      for (let i=0;i<100;++i) {const t=performance.now();await invoke('noteOn',pitch,.9,generation);note.push(performance.now()-t);await invoke('noteOff',pitch,generation)}
      const keyToWrite=[], traces=[];
      let wrote=null, ended=null, keyStart=0, trace=null;
      let skipNextRead=false, cachedState=await invoke('getParameterState');
      const remember=next=>{cachedState=next};
      backend.addEventListener('parameterStateChanged',remember);
      backend.getNativeFunction=name=>(...args)=>{
        if(trace)trace.push([name+' call',performance.now()-keyStart]);
        if(window.__latencySkipBoundary && name==='getParameterState' && skipNextRead) {
          skipNextRead=false;
          if(trace)trace.push([name+' cached boundary read',performance.now()-keyStart]);
          return Promise.resolve(cachedState);
        }
        const p=base(name)(...args);
        p.then(reply=>{
          checked(reply);
          if(name==='getParameterState')cachedState=reply;
          if(name==='beginGesture'||name==='endGesture')skipNextRead=true;
          if(name==='setParameter')skipNextRead=false;
          if(trace)trace.push([name+' ack',performance.now()-keyStart]);
          if(name==='setParameter'&&wrote) {const done=wrote;wrote=null;done(performance.now()-keyStart)}
          if(name==='endGesture'&&ended) {const done=ended;ended=null;done()}
        });
        return p;
      };
      for (let i=0;i<60;++i) {
        const ack=new Promise(r=>wrote=r), end=new Promise(r=>ended=r);
        keyStart=performance.now();
        trace=[];
        knob.dispatchEvent(new KeyboardEvent('keydown',{key:i%2?'ArrowDown':'ArrowUp',bubbles:true,cancelable:true}));
        trace.push(['handler return',performance.now()-keyStart]);
        const elapsed=await Promise.race([ack,pause(3000).then(()=>{throw Error('Knob write timeout')})]);
        keyToWrite.push(elapsed);await end;await pause(10);traces.push(trace);trace=null;
      }
      backend.getNativeFunction=base;
      backend.removeEventListener?.('parameterStateChanged',remember);
      await invoke('beginGesture',id,generation);await invoke('setParameter',id,original,generation);await invoke('endGesture',id,generation);
      report.getState=stats(roundtrip);report.writeThenRead=stats(writeRead);report.noteOnAck=stats(note);report.keyToWriteAck=stats(keyToWrite);
      report.traces=traces;
      report.skipBoundaryReads=!!window.__latencySkipBoundary;
      report.unscaled=!!window.__latencyUnscale;
      report.stage='done';
    } catch(error) {report.error=String(error);report.stage='error'}
  };
  void run();
})();
)JS";
class App final : public juce::JUCEApplication, private juce::Timer {
public:
    const juce::String getApplicationName() override {return "Dandrum Latency Probe";}
    const juce::String getApplicationVersion() override {return "1";}
    void initialise(const juce::String& args) override {
        try {
            config = args.contains("sampler") ? InstrumentDemoConfiguration::sampler() : InstrumentDemoConfiguration::tb303();
            skipBoundary=args.contains("skip-boundary");
            unscale=args.contains("unscale");
            audioProbe(config);
            processor=std::make_unique<DandrumAudioProcessor>(config);
            processor->setPlayConfigDetails(0,2,48000,128);processor->prepareToPlay(48000,128);
            editor.reset(processor->createEditor());
            auto* web=dynamic_cast<DandrumAudioProcessorEditor*>(editor.get());
            if(!web) throw std::runtime_error("No original Web editor");
            browser=&PluginEditorBridgeTestProbe::browser(*web);
            editor->addToDesktop(juce::ComponentPeer::windowHasTitleBar);editor->setVisible(true);
            deadline=now()+45000;startTimer(20);
        } catch(const std::exception& error) {std::cerr<<error.what()<<std::endl;setApplicationReturnValue(1);quit();}
    }
    void shutdown() override {stopTimer();editor.reset();processor.reset();}
private:
    InstrumentDemoConfiguration config;
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    juce::WebBrowserComponent* browser=nullptr;
    bool busy=false, finished=false, skipBoundary=false, unscale=false;double deadline=0;
    void timerCallback() override {
        if(finished || busy) return;
        if(now()>deadline) {finished=true;std::cerr<<"WebView probe timeout"<<std::endl;setApplicationReturnValue(1);quit();return;}
        busy=true;
        browser->evaluateJavascript(juce::String("window.__latencySkipBoundary=")+(skipBoundary?"true;":"false;")+"window.__latencyUnscale="+(unscale?"true;":"false;")+script, [this](auto){
            browser->evaluateJavascript("JSON.stringify(window.__latencyProbe || {})",[this](auto result){
                busy=false;
                if(result.getResult()==nullptr) return;
                const auto value=juce::JSON::parse(result.getResult()->toString());
                const auto stage=value.getProperty("stage",{}).toString();
                if(stage=="done" || stage=="error") {
                    finished=true;std::cout<<"WEB "<<config.instrumentId<<" "<<result.getResult()->toString()<<std::endl;
                    setApplicationReturnValue(stage=="done"?0:1);quit();
                }
            });
        });
    }
};
int main(int argc,char** argv) {
    std::signal(SIGPIPE,SIG_IGN);
    juce::JUCEApplicationBase::createInstance=[]()->juce::JUCEApplicationBase*{return new App;};
    return juce::JUCEApplicationBase::main(argc,const_cast<const char**>(argv));
}
