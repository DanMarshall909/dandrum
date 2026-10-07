from pathlib import Path
import json, os, subprocess
root=Path(__file__).resolve().parent
env=dict(os.environ,DISPLAY=':10');env['PATH']='/usr/bin:/bin:'+env.get('PATH','')
summary={}
for label, probe, instrument in [
 ('tb303_before','/tmp/dandrum-latency-audit-20261007/probe','tb303'),
 ('tb303_after',str(root/'probe'),'tb303'),
 ('sampler_before','/tmp/dandrum-latency-audit-20261007/probe','sampler'),
 ('sampler_after',str(root/'probe'),'sampler')]:
 result=subprocess.run([probe,instrument],env=env,capture_output=True,text=True,timeout=60)
 output=result.stdout+result.stderr
 (root/(label+'.log')).write_text(output)
 if result.returncode:raise RuntimeError(label+' failed: '+output[-1000:])
 web=[line.split(' ',2)[2] for line in result.stdout.splitlines() if line.startswith('WEB ')]
 assert len(web)==1
 report=json.loads(web[0]);assert report['stage']=='done'
 summary[label]={'exit_code':result.returncode,'web':report,'audio':[line for line in result.stdout.splitlines() if line.startswith('AUDIO ')]}
 (root/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
 print(label,json.dumps({key:value for key,value in report.items() if key!='traces'}),flush=True)
