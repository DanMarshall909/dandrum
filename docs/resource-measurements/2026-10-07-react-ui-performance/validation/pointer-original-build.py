import os, shlex, subprocess
from pathlib import Path
old=Path('/tmp/dandrum-instrument-system-ui/build/demo-webview');env=dict(os.environ);env['PATH']='/usr/bin:/bin:'+env['PATH'];env['DISPLAY']=':10'
flags={line.split(' = ',1)[0]:line.split(' = ',1)[1] for line in (old/'CMakeFiles/dandrum-sampler-plugin-host-test.dir/flags.make').read_text().splitlines() if ' = ' in line}
args=['/usr/bin/c++']
for key in ['CXX_DEFINES','CXX_INCLUDES','CXX_FLAGS']:args+=shlex.split(flags[key])
Path('/tmp/dandrum-performance-sampler-baseline.cpp').write_text('#include "/tmp/dandrum-instrument-system-ui/tests/cpp/PluginSamplerHostTest.cpp"\nextern "C" juce::AudioProcessor* __wrap__Z18createPluginFilterv() { return new DandrumAudioProcessor(InstrumentDemoConfiguration::sampler()); }\n')
args+=['-o','/tmp/dandrum-performance-sampler-baseline.o','-c','/tmp/dandrum-performance-sampler-baseline.cpp']
subprocess.run(args,cwd=old,env=env,check=True)
link=shlex.split((old/'CMakeFiles/dandrum-sampler-plugin-host-test.dir/link.txt').read_text())
link=[('/tmp/dandrum-performance-sampler-baseline.o' if x.endswith('/tests/cpp/PluginSamplerHostTest.cpp.o') else x) for x in link if '/third_party/JUCE/modules/' not in x]
link=[('dandrum-plugin_artefacts/libDandrum_SharedCode.a' if x.endswith('libDandrum Sampler_SharedCode.a') else x) for x in link]
link=[x for x in link if x != 'libdandrum-sampler-resources.a']
link+=['-Wl,--wrap=_Z18createPluginFilterv','libdandrum-tb303-react-resources.a','libdandrum-tb303-instrument-resources.a']
link[link.index('-o')+1]='/tmp/dandrum-performance-sampler-baseline'
subprocess.run(link,cwd=old,env=env,check=True)
raise SystemExit(subprocess.run(['/tmp/dandrum-performance-sampler-baseline','--web-runtime'],env=env).returncode)
