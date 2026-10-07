#include "RustEngineBindings.h"
#include <array>
#include <cmath>
#include <iostream>
#include <memory>

int main() {
    const DandrumKernelBusDeclaration buses[] {
        {"input",1,2}, {"master",2,2}, {"hp_frequency",1,1}, {"hp_q",1,1},
        {"bell_frequency",1,1}, {"bell_q",1,1}, {"bell_gain",1,1}, {"lp_frequency",1,1}, {"lp_q",1,1} };
    std::unique_ptr<DandrumKernelInstrument, decltype(&dandrum_kernel_destroy)> engine(
        dandrum_kernel_prepare_file(DANDRUM_SOURCE_ROOT "/spikes/filter-ui/filter.yaml",48000,64,buses,9), &dandrum_kernel_destroy);
    if (!engine) { std::cerr << "Filter spike graph must prepare as a stereo input effect\n"; return 1; }
    if (dandrum_patch_public_numeric_parameter_count(DANDRUM_SOURCE_ROOT "/spikes/filter-ui/filter.yaml") != 7) return 2;
    const double q = (std::sqrt(.5) - .1) / 9.9;
    const std::array<double,7> values {std::log(5.0)/std::log(400.0),q,std::log(50.0)/std::log(400.0),q,.5,1.0,q};
    for (size_t i=0;i<values.size();++i) if(!dandrum_kernel_set_public_numeric_parameter_by_slot(engine.get(),i,values[i])) return 3;
    std::array<float,64> left {},right {},outLeft {},outRight {};
    left[0]=.5f;right[0]=-.25f;
    const float* in[] {left.data(),right.data()}; float* out[] {outLeft.data(),outRight.data()};
    const DandrumKernelInputBusView input {"input",in,2,64}; const DandrumKernelOutputBusView output {"master",out,2,64};
    if (dandrum_kernel_render(engine.get(),&input,1,&output,1,64)!=64) return 4;
    // Independent RBJ high-pass(100Hz) * low-pass(8kHz) impulse coefficient at 48kHz/Q=sqrt(.5).
    const double omega=2*std::acos(-1.0)*100/48000;
    const double hp=(1+std::cos(omega))*.5/(1+std::sin(omega)/(2*std::sqrt(.5)));
    const double lp=.25/(1+std::sqrt(3.0/8.0));
    if (std::abs(outLeft[0]-.5*hp*lp)>1e-6 || std::abs(outRight[0]+.25*hp*lp)>1e-6) {
        std::cerr << "Signed stereo filter impulse differs: " << outLeft[0] << ',' << outRight[0] << '\n'; return 5;
    }
    const auto original=outLeft;
    dandrum_kernel_reset(engine.get());
    dandrum_kernel_set_public_numeric_parameter_by_slot(engine.get(),5,.5);
    dandrum_kernel_render(engine.get(),&input,1,&output,1,64);
    if (!(outLeft[0]>0 && outLeft[0]<original[0]*.1f && outRight[0]<0)) return 6;
    dandrum_kernel_reset(engine.get());
    dandrum_kernel_set_public_numeric_parameter_by_slot(engine.get(),5,1);
    dandrum_kernel_set_public_numeric_parameter_by_slot(engine.get(),4,.75);
    dandrum_kernel_render(engine.get(),&input,1,&output,1,64);
    if (!(outLeft[0]>original[0])) return 7;
    for (auto value:outLeft) if(!std::isfinite(value)) return 8;
    std::cout << "Signed stereo output, public cutoff/gain and finite filter graph passed\n";
}
