import React from 'react';
import {NumericField as SuppliedField} from '../vendor/design-system/primitives.mjs';

export function NumericField({showLabel=true,...props}){
  const decimals=Math.max(0,Math.ceil(-Math.log10(props.step??1)));
  const format=props.format??(value=>String(Number(value.toFixed(decimals))));
  const change=value=>{if(format(value)!==format(props.value))props.onChange?.(value);};
  return <span data-hide-field-label={!showLabel||undefined}><SuppliedField {...props} format={format} onChange={change}/></span>;
}
