import {layoutTreeProjection} from './projections/layout-tree.mjs';
import {sampleMappingProjection} from './projections/sample-mapping.mjs';
import {layersVoiceProjection} from './projections/layers-voice.mjs';
import {modulationProjection} from './projections/modulation.mjs';
import {routingMacrosProjection} from './projections/routing-macros.mjs';
import {inspectorHistoryProjection} from './projections/inspector-history.mjs';
import {contextMenusProjection} from './projections/context-menus.mjs';
import {frameModelProjection} from './projections/frame-model.mjs';

/** Assemble props without making presentation data part of the engine. */
export function projectReference(presenter) {
  const context={};
  layoutTreeProjection.call(presenter,context);
  sampleMappingProjection.call(presenter,context);
  layersVoiceProjection.call(presenter,context);
  modulationProjection.call(presenter,context);
  routingMacrosProjection.call(presenter,context);
  inspectorHistoryProjection.call(presenter,context);
  contextMenusProjection.call(presenter,context);
  return frameModelProjection.call(presenter,context);
}
