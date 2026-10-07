#pragma once
#include "FilterProcessor.h"
#include <cmath>
namespace filter_spike {
inline DandrumKernelInstrument* prepareKernel(double sampleRate, int frames) {
    const DandrumKernelBusDeclaration buses[] {
        {"input",1,2},{"master",2,2},{"hp_frequency",1,1},{"hp_q",1,1},
        {"bell_frequency",1,1},{"bell_q",1,1},{"bell_gain",1,1},{"lp_frequency",1,1},{"lp_q",1,1}};
    return dandrum_kernel_prepare_file(DANDRUM_SOURCE_ROOT "/spikes/filter-ui/filter.yaml",
                                      uint32_t(sampleRate), size_t(frames), buses, 9);
}
inline double kernelValue(int index, float value) {
    if(index==hpFrequency || index==bellFrequency || index==lpFrequency)
        return std::log(value/20.0)/std::log(400.0);
    if(index==bellGain) return (value+24.0)/48.0;
    return (value-.1)/9.9;
}
inline void setKernel(DandrumKernelInstrument* engine, const std::array<float,7>& values) {
    for(size_t i=0;i<values.size();++i)
        dandrum_kernel_set_public_numeric_parameter_by_slot(engine,i,kernelValue(int(i),values[i]));
}
}
