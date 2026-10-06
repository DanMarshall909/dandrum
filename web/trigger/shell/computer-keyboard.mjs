const keys=['a','w','s','e','d','f','t','g','y','h','u','j','k'];
const pages=['sample','slices','mapping','vel','voice','mod','routing','fx'];
export function installComputerKeyboard({onNoteOn,onNoteOff,onPage,onAudition,onEscape,onModulation}){
  let octave=3;const held=new Map();
  const release=()=>{for(const note of held.values())onNoteOff(note);held.clear();};
  const down=e=>{
    if(e.target?.closest?.('input,textarea,select,[contenteditable="true"],[role="dialog"]'))return;
    const key=e.key.toLowerCase();
    if((e.ctrlKey||e.metaKey)&&/^[1-8]$/.test(key)){e.preventDefault();onPage(pages[+key-1]);return;}
    if(e.ctrlKey||e.metaKey||e.altKey)return;
    if(key==='escape'){release();onEscape();return;}
    if(key==='m'){onModulation(e.target);return;}
    if(key===' '){e.preventDefault();if(!e.repeat)onAudition();return;}
    if(key==='z'||key==='x'){if(!e.repeat)octave=Math.max(0,Math.min(8,octave+(key==='x'?1:-1)));return;}
    const index=keys.indexOf(key);if(index<0||e.repeat||held.has(key))return;
    e.preventDefault();const note=Math.min(127,(octave+1)*12+index);held.set(key,note);onNoteOn(note,e.shiftKey?127:100);
  };
  const up=e=>{const key=e.key.toLowerCase(),note=held.get(key);if(note!=null){held.delete(key);onNoteOff(note);}};
  window.addEventListener('keydown',down);window.addEventListener('keyup',up);window.addEventListener('blur',release);
  return ()=>{release();window.removeEventListener('keydown',down);window.removeEventListener('keyup',up);window.removeEventListener('blur',release);};
}
