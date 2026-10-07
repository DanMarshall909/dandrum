#include "PluginProcessor.h"
#include "PluginEditor.h"
#include <atomic>
#include <chrono>
#include <csignal>
#include <iostream>
#include <memory>
#include <thread>

struct PluginEditorBridgeTestProbe {
    static juce::WebBrowserComponent& browser(DandrumAudioProcessorEditor& editor) { return editor.browser; }
};
using Clock = std::chrono::steady_clock;
static double milliseconds() { return juce::Time::getMillisecondCounterHiRes(); }
static constexpr auto exerciseScript = R"JS(
(() => {
  const knob=document.querySelector('[data-parameter-id][aria-disabled="false"]');
  if(!knob) throw Error('No ready original React knob');
  const backend=window.__JUCE__.backend, base=backend.getNativeFunction;
  const report=window.__resourceExercise={events:0,writes:0,rejected:0,closed:false};
  backend.getNativeFunction=name=>(...args)=>{
    const p=base(name)(...args);
    p.then(reply=>{
      if(name==='setParameter') {
        if(typeof reply==='string'||reply?.status!=='accepted')++report.rejected;
        else ++report.writes;
      }
    });return p;
  };
  let index=0;
  const timer=setInterval(()=>{
    knob.dispatchEvent(new KeyboardEvent('keydown',{
      key:index++%2?'ArrowDown':'ArrowUp',bubbles:true,cancelable:true}));
    ++report.events;
  },1000/30);
  setTimeout(()=>{clearInterval(timer);report.closed=true},10000);
  return true;
})();
)JS";
class App final : public juce::JUCEApplication, private juce::Timer {
public:
    const juce::String getApplicationName() override { return "Dandrum Resource Probe"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise(const juce::String& args) override {
        try {
            config=args.contains("sampler")?InstrumentDemoConfiguration::sampler():InstrumentDemoConfiguration::tb303();
            unscaled=args.contains("unscale");
            processor=std::make_unique<DandrumAudioProcessor>(config);
            processor->setPlayConfigDetails(0,2,48000,128);processor->prepareToPlay(48000,128);
            if(!processor->isInstrumentLoaded())throw std::runtime_error(processor->getLastLoadError().toStdString());
            processor->setFileWatchEnabled(false);
            running=true;
            audio=std::thread([this]{
                juce::AudioBuffer<float> buffer(2,128);juce::MidiBuffer midi;bool playing=false;
                auto due=Clock::now();size_t counter=0;
                while(running) {
                    midi.clear();const bool next=sounding.load();
                    const int note=config.instrumentId=="dandrum.tb303-acid"?60:36;
                    if(next&&!playing)midi.addEvent(juce::MidiMessage::noteOn(1,static_cast<juce::uint8>(note),.9f),0);
                    if(!next&&playing)midi.addEvent(juce::MidiMessage::noteOff(1,note),0);
                    if(next&&config.instrumentId!="dandrum.tb303-acid"&&counter++%75==0)
                        midi.addEvent(juce::MidiMessage::noteOn(1,static_cast<juce::uint8>(note),.9f),0);
                    playing=next;processor->processBlock(buffer,midi);++blocks;
                    for(int i=0;i<128;++i)if(std::abs(buffer.getSample(0,i))>1e-7f){++signalBlocks;break;}
                    due+=std::chrono::nanoseconds(2666667);std::this_thread::sleep_until(due);
                }
            });
            phase("engine_only");stage=0;startTimer(100);
        }catch(const std::exception& error){fail(error.what());}
    }
    void shutdown() override {
        stopTimer();running=false;if(audio.joinable())audio.join();editor.reset();processor.reset();
        std::cout<<"RESULT {\"blocks\":"<<blocks<<",\"signal_blocks\":"<<signalBlocks<<"}"<<std::endl;
    }
private:
    InstrumentDemoConfiguration config;
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    juce::WebBrowserComponent* browser=nullptr;
    std::thread audio;
    std::atomic<bool> running{false},sounding{false};
    std::atomic<size_t> blocks{0},signalBlocks{0};
    int stage=0;bool busy=false,unscaled=false;double started=0,loadDeadline=0;
    void phase(const char* name) {
        started=milliseconds();std::cout<<"PHASE "<<name<<" "<<config.instrumentId<<" unscaled="<<unscaled<<std::endl;
    }
    void fail(const std::string& error) {std::cerr<<"ERROR "<<error<<std::endl;setApplicationReturnValue(1);quit();}
    void timerCallback() override {
        if(busy)return;
        const auto elapsed=milliseconds()-started;
        if(stage==0&&elapsed>=8000) {
            editor.reset(processor->createEditor());
            auto* web=dynamic_cast<DandrumAudioProcessorEditor*>(editor.get());
            if(!web){fail("Original Web editor unavailable");return;}
            browser=&PluginEditorBridgeTestProbe::browser(*web);
            editor->addToDesktop(juce::ComponentPeer::windowHasTitleBar);editor->setVisible(true);
            phase("loading");loadDeadline=milliseconds()+20000;stage=1;
        }else if(stage==1) {
            if(milliseconds()>loadDeadline){fail("React controls failed to load");return;}
            busy=true;
            browser->evaluateJavascript("Boolean(document.querySelector('[data-parameter-id][aria-disabled=\"false\"]'))",[this](auto result){
                busy=false;
                if(!result.getResult()||!static_cast<bool>(*result.getResult()))return;
                if(unscaled)browser->evaluateJavascript("(()=>{const s=document.createElement('style');s.textContent='.machine{transform:none!important}';document.head.append(s);return true})()",{});
                phase("editor_idle");stage=2;
            });
        }else if(stage==2&&elapsed>=10000) {
            sounding=true;busy=true;
            browser->evaluateJavascript(exerciseScript,[this](auto result){
                busy=false;
                if(result.getError()||!result.getResult()){fail("React control exercise failed to start");return;}
                phase("editor_active");stage=3;
            });
        }else if(stage==3&&elapsed>=11000) {
            busy=true;
            browser->evaluateJavascript("JSON.stringify(window.__resourceExercise)",[this](auto result){
                busy=false;
                if(!result.getResult()){fail("Missing control exercise report");return;}
                const auto report=juce::JSON::parse(result.getResult()->toString());
                std::cout<<"EXERCISE "<<result.getResult()->toString()<<std::endl;
                if(static_cast<int>(report.getProperty("writes",0))<10||static_cast<int>(report.getProperty("rejected",0))!=0){fail("Native writes were missing or rejected");return;}
                sounding=false;phase("settling");stage=4;
            });
        }else if(stage==4&&elapsed>=3000) {
            editor.reset();browser=nullptr;phase("editor_closed");stage=5;
        }else if(stage==5&&elapsed>=8000) {stage=6;quit();}
    }
};
int main(int argc,char** argv) {
    std::signal(SIGPIPE,SIG_IGN);
    juce::JUCEApplicationBase::createInstance=[]()->juce::JUCEApplicationBase*{return new App;};
    return juce::JUCEApplicationBase::main(argc,const_cast<const char**>(argv));
}
