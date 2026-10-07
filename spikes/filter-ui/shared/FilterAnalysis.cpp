#include "FilterViewModel.h"
#include "FilterKernel.h"
namespace filter_spike {
void FilterViewModel::updateResponse() {
    const double sampleRate=processor.analysisSampleRate();
    if(previousRate!=sampleRate || !responseEngine) {
        if(responseEngine) dandrum_kernel_destroy(responseEngine);
        responseEngine=prepareKernel(sampleRate,8192);previousRate=sampleRate;
    }
    if(!responseEngine) { visual.responseDb.fill(-90);return; }
    dandrum_kernel_reset(responseEngine);setKernel(responseEngine,previous);
    impulse.fill(0);impulse[0]=1;
    const float* inputs[]{impulse.data(),impulse.data()};
    float* outputs[]{responseLeft.data(),responseRight.data()};
    const DandrumKernelInputBusView inputView{"input",inputs,2,8192};
    const DandrumKernelOutputBusView outputView{"master",outputs,2,8192};
    if(dandrum_kernel_render(responseEngine,&inputView,1,&outputView,1,8192)!=8192) {
        visual.responseDb.fill(-90);return;
    }
    responseTransform.fill(0);std::copy(responseLeft.begin(),responseLeft.end(),responseTransform.begin());
    responseFft.performFrequencyOnlyForwardTransform(responseTransform.data());
    for(int i=0;i<plotBins;++i) {
        const float bin=juce::jlimit(1.0f,4095.0f,frequencyAt(float(i)/(plotBins-1))*8192/float(sampleRate));
        const int lower=int(bin);const float mix=bin-float(lower);
        const float magnitude=responseTransform[size_t(lower)]*(1-mix)+responseTransform[size_t(lower+1)]*mix;
        visual.responseDb[size_t(i)]=juce::Decibels::gainToDecibels(magnitude,-90.0f);
    }
}
void FilterViewModel::updateSpectrum() {
    const auto analyze=[this](const auto& samples,auto& destination) {
        transform.fill(0);
        for(size_t i=0;i<samples.size();++i) transform[i]=samples[i]*window[i];
        fft.performFrequencyOnlyForwardTransform(transform.data());
        for(int i=0;i<plotBins;++i) {
            const float bin=juce::jlimit(1.0f,1023.0f,frequencyAt(float(i)/(plotBins-1))*fftSize/float(previousRate));
            const int lower=int(bin);const float mix=bin-float(lower);
            const float magnitude=(transform[size_t(lower)]*(1-mix)+transform[size_t(lower+1)]*mix)*4/fftSize;
            destination[size_t(i)]=juce::Decibels::gainToDecibels(magnitude,-90.0f);
        }
    };
    analyze(input,visual.inputDb);analyze(output,visual.outputDb);
    for(int i=0;i<plotBins;++i) visual.historyDb[size_t(visual.historyHead*plotBins+i)]=visual.outputDb[size_t(i)];
    visual.historyHead=(visual.historyHead+1)%historyColumns;
}
}
