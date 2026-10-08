import React from 'react';
import * as DD from './design-system/index.jsx';
export function PropertyEditor({row}){
  if(!row.field)return <DD.PropertyRow label={row.label} value={row.value} unit={row.unit} prepared={row.ro} hint={row.hint} compact/>;
  return <DD.PropertyRow label={row.label} compact value={<span className="property-editor">
    {row.field.options?<DD.MenuButton label={row.label} showLabel={false} {...row.field} width={110} compact/>:
      <DD.NumericField label={row.label} showLabel={false} {...row.field} unit={row.unit} width={110} compact/>}
  </span>}/>;
}
