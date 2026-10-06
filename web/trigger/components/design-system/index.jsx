// Public entry point for the received design-system library.
import {installControlAdapters} from '../../vendor/design-system/primitives.mjs';
import {ParameterKnob} from '../ParameterKnob.jsx';
import {NumericField} from '../NumericField.jsx';
import {Toggle} from '../Toggle.jsx';
import {ChoiceMenu} from '../ChoiceMenu.jsx';
installControlAdapters({Knob:ParameterKnob,NumericField,Toggle,MenuButton:ChoiceMenu});
export * from '../../vendor/design-system/primitives.mjs';
export {ParameterKnob as Knob} from '../ParameterKnob.jsx';
export {ChoiceMenu as MenuButton} from '../ChoiceMenu.jsx';
export {Toggle} from '../Toggle.jsx';
export {NumericField} from '../NumericField.jsx';
