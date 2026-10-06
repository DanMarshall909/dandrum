import React from 'react';
import * as DD from './design-system/index.jsx';
export function GenericMenu(props){return <DD.ContextMenu {...props}/>;}
export function ControlMenu(props){return <GenericMenu {...props}/>;}
export function DestinationMenu(props){return <GenericMenu {...props}/>;}
