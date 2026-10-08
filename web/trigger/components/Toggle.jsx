import React from 'react';
import {Toggle as SuppliedToggle} from '../vendor/design-system/primitives.mjs';

/** Keep the switch's accessible name when its table cell has no visible caption. */
export function Toggle({showLabel=true,...props}){
  return <span data-hide-toggle-label={!showLabel||undefined}><SuppliedToggle {...props}/></span>;
}
