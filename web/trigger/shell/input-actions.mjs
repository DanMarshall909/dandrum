import React from 'react';

export const inputActions = {
attach() {
    this.mapRef = this.mapRef || React.createRef(); this.winRef = this.winRef || React.createRef();
    this.keyboardBindings={
      onNoteOn:(note,velocity)=>this.props.store.noteOn(note,velocity),onNoteOff:note=>this.props.store.noteOff(note),
      onPage:page=>this.go(page),onAudition:()=>this.auditionSelection(),
      onEscape:()=>this.setState({dialog:null,importDialog:null,cmenu:null,kmenu:null,dmenu:null,menu:null,dragSrc:null,dragMod:null,rail:null}),
      onModulation:target=>{const knob=target?.closest?.('[data-knob]');if(knob){const r=knob.getBoundingClientRect();const p=this.winPt({clientX:r.left,clientY:r.bottom},264,200);this.setState({kmenu:{...p,label:knob.getAttribute('data-knob'),path:[]}});}},
    };this.forceUpdate();
    this.onR = () => this.forceUpdate();
    this.onMod = (e) => { const s = !!e.shiftKey; if (s !== this.shift) { this.shift = s; if (this.snap) this.setState({ dragShift: s }); } };
    this.onDown = e => {
      const target=e.target?.closest?.('[data-knob],[data-reset^="macro|"],[role="slider"],[role="spinbutton"]');
      if(target&&e.button===0&&!e.altKey){this.controlGesture=target.getAttribute('data-knob')??target.getAttribute('data-reset')??target.getAttribute('aria-label');this.props.store.beginGesture(this.controlGesture);}
    };
    this.onUp = () => {
      if(this.controlGesture){const id=this.controlGesture;queueMicrotask(()=>this.props.store.endGesture(id));this.controlGesture=null;}
      if(this.snap){const before=this.snap,next=this.getZones();this.snap=null;this.setState({dragShift:false});
        if(JSON.stringify(before)!==JSON.stringify(next))this.commit(next,'Edit mapping');else this.setState({zones:null});}
    };
    this.onCancel = () => {if(this.controlGesture)Promise.resolve(this.props.store.cancelGesture()).catch(error=>this.setState({modNote:error.message}));this.controlGesture=null;this.snap=null;this.setState({zones:null,dragShift:false});};
    this.onKey = (e) => {
      if(e.target?.closest?.('[role="dialog"]'))return;
      this.onMod(e);
      if (e.type === 'keydown' && e.key === 'Escape' && (this.state.kmenu || this.state.dragSrc)) { this.setState({ kmenu: null, dragSrc: null, nodeOver: null }); return; }
      if (e.type === 'keydown' && (e.ctrlKey || e.metaKey) && !(e.target && /INPUT|TEXTAREA/.test(e.target.tagName))) {
        const kz = e.key.toLowerCase();
        if(kz==='o'){e.preventDefault();this.browseSamples?.();return;}
        if (kz === 'z') { e.preventDefault(); this.undoStep(e.shiftKey ? 1 : -1); return; }
        if (kz === 'y') { e.preventDefault(); this.undoStep(1); return; }
      }
      if (e.type === 'keydown' && e.key === 'Escape' && (this.state.cmenu || this.state.dmenu)) { this.setState({ cmenu: null, dmenu: null }); return; }
      const treeTarget=e.target?.closest?.('[data-node]')?.dataset.node;if(e.type==='keydown'&&treeTarget&&!e.target?.closest?.('input,textarea')){if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='d'){e.preventDefault();this.command('Duplicate node','duplicateNode',treeTarget);return;}if(e.key==='Delete'){e.preventDefault();this.command('Delete node','removeNode',treeTarget);return;}}
      if(e.type==='keydown'&&this.curPage==='slices'&&!e.ctrlKey&&!e.metaKey&&!e.target?.closest?.('input,textarea,[role="slider"]')){
        if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();this.sliceKeyboard?.step(e.key==='ArrowLeft'?-1:1);}
        if(e.key.toLowerCase()==='m'&&!e.target?.closest?.('[data-knob]')){e.preventDefault();this.sliceKeyboard?.add();}
        if(e.key==='Delete'||e.key==='Backspace'){e.preventDefault();this.sliceKeyboard?.remove();}
        return;
      }
      if(e.type==='keydown'&&this.curPage==='layers'&&!e.ctrlKey&&!e.metaKey&&!e.target?.closest?.('input,textarea,[role="slider"],[role="spinbutton"]')){
        if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();this.candidateKeyboard?.step(e.key==='ArrowLeft'?-1:1);}
        if(e.key==='Delete'||e.key==='Backspace'){e.preventDefault();this.candidateKeyboard?.remove();}return;
      }
      if(e.type==='keydown'&&this.curPage==='mod'&&e.key==='Delete'&&!e.target?.closest?.('input,textarea,[role="slider"]')){e.preventDefault();this.routeKeyboard?.remove();}
      if (e.type !== 'keydown' || this.curPage !== 'mapping') return;
      const t = e.target; if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA')) return;
      if (e.key === 'Escape' && this.state.menu) { this.setState({ menu: null }); return; }
      const sel = this.getZones().find(z => z.id === this.state.zoneSel);
      const mod = e.ctrlKey || e.metaKey;
      if(mod&&e.key.toLowerCase()==='d'){e.preventDefault();this.mappingKeyboard?.duplicate();}
      else if(!mod&&(e.key==='ArrowLeft'||e.key==='ArrowRight')&&!t?.closest?.('[role="slider"]')){e.preventDefault();this.mappingKeyboard?.step(e.key==='ArrowLeft'?-1:1);}
      else if (mod && e.key.toLowerCase() === 'c' && sel) { e.preventDefault(); this.copyRange(sel); }
      else if (mod && e.key.toLowerCase() === 'v' && this.state.clip) { e.preventDefault(); this.paste(this.hoverNote ?? (sel ? sel.hi + 1 : 60), e.shiftKey); }
      else if ((e.key === 'Delete' || e.key === 'Backspace') && sel && !mod) { e.preventDefault(); this.del(sel, e.shiftKey); }
    };
    window.addEventListener('resize', this.onR);
    window.addEventListener('keydown', this.onKey); window.addEventListener('keyup', this.onKey);
    window.addEventListener('pointermove', this.onMod, true); window.addEventListener('pointerdown', this.onMod, true);
    window.addEventListener('pointerup', this.onUp, true);
    window.addEventListener('pointercancel', this.onCancel, true);
    window.addEventListener('blur',this.onCancel);
    window.addEventListener('pointerdown', this.onDown, true);
  },
detach() {
    this.velocityCleanup?.();
    this.releaseKeyboard?.();clearTimeout(this.flashT);
    window.removeEventListener('resize', this.onR);
    window.removeEventListener('keydown', this.onKey); window.removeEventListener('keyup', this.onKey);
    window.removeEventListener('pointermove', this.onMod, true); window.removeEventListener('pointerdown', this.onMod, true);
    window.removeEventListener('pointerup', this.onUp, true);
    window.removeEventListener('pointercancel', this.onCancel, true);
    window.removeEventListener('blur',this.onCancel);
    window.removeEventListener('pointerdown', this.onDown, true);
    if(this.controlGesture)this.props.store.endGesture(this.controlGesture);
    clearInterval(this.pvT);
  },
onVelDown(e) {
    const t = e.target.closest && e.target.closest('[data-vh]'); if (!t) return;
    e.preventDefault();
    const mode = t.getAttribute('data-vh'), rect = e.currentTarget.getBoundingClientRect();
    const st0 = { lo: this.state.vHardLo ?? 90, hi: this.state.vSoftHi ?? 101 };
    const v0 = 1 + (e.clientX - rect.left) / rect.width * 126;
    const mv = (ev) => {
      const v = 1 + (ev.clientX - rect.left) / rect.width * 126, cl = (x, a, b) => Math.max(a, Math.min(b, Math.round(x)));
      if (mode === 'l') this.setState({ vHardLo: cl(v, 2, st0.hi + 1) });
      else if (mode === 'r') this.setState({ vSoftHi: cl(v, (this.state.vHardLo ?? st0.lo) - 1, 126) });
      else { let d = Math.round(v - v0); d = Math.max(2 - st0.lo, Math.min(126 - st0.hi, d)); this.setState({ vHardLo: st0.lo + d, vSoftHi: st0.hi + d }); }
    };
    const up = () => { window.removeEventListener('pointermove', mv); window.removeEventListener('pointerup', up); };
    window.addEventListener('pointermove', mv); window.addEventListener('pointerup', up);
  }
};
