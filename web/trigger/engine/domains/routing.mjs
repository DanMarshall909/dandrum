import {clone,entity,finite} from '../validation.mjs';
export const routingOperations={
  setChain(p,id,delta){const chain=entity(p.chains,id);if(delta.output)entity(p.buses,delta.output);if(delta.level!=null)finite(delta.level);Object.assign(chain,clone(delta),{id});},
};
