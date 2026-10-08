import React,{useLayoutEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {MenuButton as SuppliedButton,ContextMenu} from '../vendor/design-system/primitives.mjs';

/** The supplied MenuButton is a drawing; this wrapper owns choice and placement. */
export function ChoiceMenu({options,onChange,value,showLabel=true,...props}){
  const anchor=useRef(null),[popup,setPopup]=useState(null);
  useLayoutEffect(()=>{const button=anchor.current?.querySelector('button');if(button&&props.label)button.setAttribute('aria-label',props.label);},[props.label,value]);
  const close=()=>{setPopup(null);anchor.current?.querySelector('button')?.focus();};
  const open=()=>{
    if(popup){close();return;}
    const button=anchor.current.querySelector('button'),editor=button.closest('[data-testid=trigger-editor],[role=dialog],[data-story-canvas]')??document.body;
    if(!editor)return;
    const bounds=editor.getBoundingClientRect(),rect=button.getBoundingClientRect(),width=Math.max(180,props.width??180),height=Math.min(bounds.height-8,options.length*26+40);
    const left=Math.max(4,Math.min(rect.left-bounds.left,bounds.width-width-4));
    const top=rect.bottom-bounds.top+2+height>bounds.height?Math.max(4,rect.top-bounds.top-height-2):rect.bottom-bounds.top+2;
    setPopup({editor,left,top,width,height});
  };
  if(!options)return <SuppliedButton {...props} value={value}/>;
  const choices=options.map(option=>typeof option==='string'?{id:option,label:option}:option);
  return <>
    <span ref={anchor} style={{display:'contents'}}>
      <SuppliedButton {...props} label={showLabel?props.label:undefined} value={choices.find(option=>option.id===value)?.label??value} onClick={open}/>
    </span>
    {popup&&createPortal(<>
      <div style={{position:'absolute',inset:0,zIndex:48}} onPointerDown={close}/>
      <div style={{position:'absolute',left:popup.left,top:popup.top,zIndex:49}}>
        <ContextMenu title={props.label} width={popup.width} onClose={close} style={{maxHeight:popup.height,overflowY:'auto'}}
          items={choices.map(option=>({label:option.label,checked:option.id===value,disabled:option.disabled,
            onSelect:()=>{onChange?.(option.id);close();}}))}/>
      </div>
    </>,popup.editor)}
  </>;
}
