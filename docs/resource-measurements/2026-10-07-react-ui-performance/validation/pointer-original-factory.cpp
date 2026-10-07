#include "/tmp/dandrum-instrument-system-ui/tests/cpp/PluginSamplerHostTest.cpp"
extern "C" juce::AudioProcessor* __wrap__Z18createPluginFilterv() { return new DandrumAudioProcessor(InstrumentDemoConfiguration::sampler()); }
