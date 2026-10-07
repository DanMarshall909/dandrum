from pathlib import Path
import json, os, queue, statistics, subprocess, threading, time

ROOT = Path('/tmp/dandrum-optimized-audit-20261007/resource')
HZ = os.sysconf('SC_CLK_TCK')

def process_info(pid):
    try:
        base = Path('/proc') / str(pid)
        stat = (base / 'stat').read_text().rsplit(')', 1)[1].split()
        totals = {}
        for line in (base / 'smaps_rollup').read_text().splitlines():
            if ':' in line:
                key, rest = line.split(':', 1)
                if rest.strip().split() and rest.strip().split()[0].isdigit():
                    totals[key] = int(rest.strip().split()[0])
        return dict(pid=pid, start=int(stat[19]), ticks=int(stat[11])+int(stat[12]),
                    name=(base/'comm').read_text().strip(),
                    pss_kib=totals['Pss'], rss_kib=totals['Rss'],
                    private_kib=totals.get('Private_Clean',0)+totals.get('Private_Dirty',0),
                    swap_pss_kib=totals.get('SwapPss',0))
    except (FileNotFoundError, ProcessLookupError):
        return None

def descendants(pid):
    seen, pending = set(), [pid]
    while pending:
        parent=pending.pop()
        if parent in seen: continue
        seen.add(parent)
        for child_file in (Path('/proc')/str(parent)/'task').glob('*/children'):
            try: pending.extend(int(item) for item in child_file.read_text().split())
            except (FileNotFoundError, ProcessLookupError): pass
    return seen

def percentile(values, q):
    values=sorted(values)
    return values[int((len(values)-1)*q)]

def measure(label, argv):
    env=dict(os.environ);env['PATH']='/usr/bin:/bin:'+env.get('PATH','');env['DISPLAY']=':10'
    process=subprocess.Popen(argv,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,env=env)
    events=queue.Queue(); lines=[]
    def reader():
        for line in process.stdout:
            lines.append(line); events.put((time.monotonic(),line))
    thread=threading.Thread(target=reader,daemon=True);thread.start()
    phase='starting'; phase_start=time.monotonic(); start=phase_start
    previous={};previous_time=start; tracked={}; samples=[];reports=[]
    while process.poll() is None:
        if time.monotonic()-start>80:
            process.terminate();raise RuntimeError(label+' exceeded bounded probe runtime')
        while not events.empty():
            stamp,line=events.get_nowait()
            if line.startswith('PHASE '):
                phase=line.split()[1];phase_start=stamp;print(label,phase,flush=True)
            if line.startswith('EXERCISE ') or line.startswith('RESULT '):reports.append(line.strip())
        current={}
        for pid in descendants(process.pid) | set(tracked):
            info=process_info(pid)
            if info is None: continue
            if pid in tracked and info['start']!=tracked[pid]:continue
            tracked[pid]=info['start'];current[pid]=info
        stamp=time.monotonic();elapsed=stamp-previous_time
        ticks=sum(max(0,info['ticks']-previous[pid]['ticks']) for pid,info in current.items()
                  if pid in previous and previous[pid]['start']==info['start'])
        sample=dict(time_s=stamp-start,phase=phase,phase_s=stamp-phase_start,
                    cpu_percent_one_core=100*ticks/HZ/elapsed,
                    pss_mib=sum(p['pss_kib'] for p in current.values())/1024,
                    rss_sum_mib=sum(p['rss_kib'] for p in current.values())/1024,
                    private_mib=sum(p['private_kib'] for p in current.values())/1024,
                    swap_pss_mib=sum(p['swap_pss_kib'] for p in current.values())/1024,
                    processes=list(current.values()))
        samples.append(sample);previous=current;previous_time=stamp
        time.sleep(.5)
    thread.join(timeout=2)
    for stamp,line in list(events.queue):
        if line.startswith('EXERCISE ') or line.startswith('RESULT '):reports.append(line.strip())
    (ROOT/(label+'.log')).write_text(''.join(lines))
    summary={}
    for stage in ['engine_only','editor_idle','editor_active','editor_closed']:
        phase_rows=[sample for sample in samples if sample['phase']==stage]
        end=max(sample['phase_s'] for sample in phase_rows)
        rows=[sample for sample in phase_rows if 2 <= sample['phase_s'] < end - 1]
        if len(rows)<6:raise RuntimeError(label+' lacked steady samples for '+stage)
        if stage.startswith('editor_') and stage!='editor_closed' and not any(
                'WebKitWeb' in p['name'] for row in rows for p in row['processes']):
            raise RuntimeError(label+' failed to count the WebKit renderer')
        summary[stage]=dict(samples=len(rows),cpu_mean_one_core=statistics.mean(r['cpu_percent_one_core'] for r in rows),
            cpu_p95_one_core=percentile([r['cpu_percent_one_core'] for r in rows],.95),
            pss_mean_mib=statistics.mean(r['pss_mib'] for r in rows),pss_max_mib=max(r['pss_mib'] for r in rows),
            rss_sum_mean_mib=statistics.mean(r['rss_sum_mib'] for r in rows),
            private_mean_mib=statistics.mean(r['private_mib'] for r in rows),
            process_names=sorted({p['name'] for r in rows for p in r['processes']}),
            max_processes=max(len(r['processes']) for r in rows))
    result=dict(label=label,argv=argv,exit_code=process.returncode,summary=summary,reports=reports,samples=samples)
    (ROOT/(label+'.json')).write_text(json.dumps(result,indent=2))
    if process.returncode:raise RuntimeError(label+' probe failed: '+''.join(lines[-5:]))
    print(json.dumps(dict(label=label,exit_code=process.returncode,summary=summary,reports=reports)),flush=True)
    return result

if __name__=='__main__':
    results={}
    for label,probe,args in [('tb303_before','/tmp/dandrum-resource-audit-20261007/probe',['tb303']),('tb303_after',str(ROOT/'probe'),['tb303']),('sampler_before','/tmp/dandrum-resource-audit-20261007/probe',['sampler']),('sampler_after',str(ROOT/'probe'),['sampler'])]:
        result=measure(label,[probe,*args])
        results[label]={'summary':result['summary'],'reports':result['reports']}
        (ROOT/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
