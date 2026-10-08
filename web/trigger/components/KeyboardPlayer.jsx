import React,{useEffect} from 'react';
import {installComputerKeyboard} from '../shell/computer-keyboard.mjs';
export function KeyboardPlayer({bindings,visible=false}){
  useEffect(()=>bindings?installComputerKeyboard(bindings):undefined,[bindings]);
  return visible?<p style={{font:'500 12px var(--font-ui)',color:'var(--dd-paper-2)'}}>A W S E D F T G Y H U J K · Z/X shifts octaves · Space previews</p>:null;
}
