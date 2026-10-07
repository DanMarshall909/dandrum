#include "FilterKernel.h"
namespace filter_spike {
void FilterProcessor::prepareToPlay(double sampleRate,int frames) {
    releaseResources(); rate.store(sampleRate);
    inputBuffer.setSize(2,std::max(1,frames));
    engine=prepareKernel(sampleRate,std::max(1,frames));
    applied.fill(std::numeric_limits<float>::quiet_NaN());
    setLatencySamples(engine ? int(dandrum_kernel_total_latency_samples(engine)) : 0);
}
void FilterProcessor::releaseResources() { if(engine) dandrum_kernel_destroy(engine);engine=nullptr; }
void FilterProcessor::processBlock(juce::AudioBuffer<float>& buffer,juce::MidiBuffer&) {
    juce::ScopedNoDenormals noDenormals;
    if(buffer.getNumChannels()<2 || !engine) { buffer.clear();return; }
    std::array<float,7> next{};
    for(size_t i=0;i<next.size();++i) next[i]=actual(int(i));
    if(next!=applied) { setKernel(engine,next);applied=next; }
    const bool testSignal=actual(audition)>.5f, dry=actual(bypass)>.5f;
    std::array<float,4> callbackPeaks {};
    for(int offset=0;offset<buffer.getNumSamples();offset+=inputBuffer.getNumSamples()) {
        const int frames=std::min(inputBuffer.getNumSamples(),buffer.getNumSamples()-offset);
        if(!testSignal) for(int channel=0;channel<2;++channel)
            inputBuffer.copyFrom(channel,0,buffer,channel,offset,frames);
        else for(int i=0;i<frames;++i) {
            noiseState^=noiseState<<13;noiseState^=noiseState>>17;noiseState^=noiseState<<5;
            const float signal=.08f*(float(noiseState)/float(UINT32_MAX)*2-1)+.08f*float(std::sin(phase));
            phase+=2*juce::MathConstants<double>::pi*440/rate.load();
            if(phase>2*juce::MathConstants<double>::pi) phase-=2*juce::MathConstants<double>::pi;
            for(int channel=0;channel<2;++channel)
                inputBuffer.setSample(channel,i,signal);
        }
        const float* inputs[]{inputBuffer.getReadPointer(0),inputBuffer.getReadPointer(1)};
        float* outputs[]{buffer.getWritePointer(0,offset),buffer.getWritePointer(1,offset)};
        if(dry) for(int channel=0;channel<2;++channel)
            juce::FloatVectorOperations::copy(outputs[channel],inputs[channel],frames);
        else {
            const DandrumKernelInputBusView input{"input",inputs,2,size_t(frames)};
            const DandrumKernelOutputBusView output{"master",outputs,2,size_t(frames)};
            if(dandrum_kernel_render(engine,&input,1,&output,1,size_t(frames))!=size_t(frames))
                for(auto* channel:outputs) juce::FloatVectorOperations::clear(channel,frames);
        }
        for(int channel=0;channel<2;++channel) {
            float before=0,after=0;
            for(int i=0;i<frames;++i) { before=std::max(before,std::abs(inputs[channel][i]));
                after=std::max(after,std::abs(outputs[channel][i])); }
            callbackPeaks[size_t(channel)]=std::max(callbackPeaks[size_t(channel)],before);
            callbackPeaks[size_t(channel+2)]=std::max(callbackPeaks[size_t(channel+2)],after);
        }
        int a,n,b,m;fifo.prepareToWrite(frames,a,n,b,m);
        for(int i=0;i<n+m;++i) { const auto slot=size_t(i<n?a+i:b+i-n);
            captureInput[slot]=(inputs[0][i]+inputs[1][i])*.5f;
            captureOutput[slot]=(outputs[0][i]+outputs[1][i])*.5f; }
        fifo.finishedWrite(n+m); // Drop visual samples when full; audio never waits for a UI.
    }
    for(size_t i=0;i<callbackPeaks.size();++i) peaks[i].store(callbackPeaks[i]);
}
}
