import json, os, runpy, subprocess, time
from pathlib import Path
root=Path(__file__).resolve().parent
observer=runpy.run_path(str(root/'resource/measure.py'))
child=subprocess.Popen(['python3','-u','-c', """import sys,time
print('READY',flush=True);sys.stdin.readline()
allocation=bytearray(32*1024*1024)
for i in range(0,len(allocation),4096):allocation[i]=1
print('ALLOCATED',flush=True);sys.stdin.readline()
print('BUSY',flush=True)
due=time.monotonic()+3
while time.monotonic()<due:pass
print('DONE',flush=True);sys.stdin.readline()
"""],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
try:
 assert child.stdout.readline().strip()=='READY'
 before=observer['process_info'](child.pid)
 child.stdin.write('allocate\n');child.stdin.flush()
 assert child.stdout.readline().strip()=='ALLOCATED'
 allocated=observer['process_info'](child.pid)
 child.stdin.write('busy\n');child.stdin.flush()
 assert child.stdout.readline().strip()=='BUSY'
 started=time.monotonic();initial=observer['process_info'](child.pid)
 assert child.stdout.readline().strip()=='DONE'
 elapsed=time.monotonic()-started;end=observer['process_info'](child.pid)
 increase=(allocated['pss_kib']-before['pss_kib'])/1024
 cpu=100*(end['ticks']-initial['ticks'])/os.sysconf('SC_CLK_TCK')/elapsed
 result={'date':'2026-10-07','known_allocation_mib':32,'measured_pss_increase_mib':increase,
         'busy_loop_cpu_percent_one_core':cpu,'pass':31.5<increase<32.5 and 95<cpu<105}
 (root/'calibration.json').write_text(json.dumps(result,indent=2)+'\n');print(result)
 assert result['pass']
finally:
 child.stdin.write('close\n');child.stdin.flush();child.wait(timeout=5)
