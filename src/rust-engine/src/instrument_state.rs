use std::collections::BTreeMap;

use crate::patch::{ParameterValue, PatchDocument, PortReference, PresetParameterTargetDeclaration};

/// Immutable instrument structure captured at load time.
///
/// Runtime public-parameter values live in `InstrumentParameterState` instead;
/// changing those values must not mutate this definition or rewrite the YAML it
/// came from.
#[derive(Clone, Debug, PartialEq)]
pub struct InstrumentDefinition {
    pub patch_doc: PatchDocument,
    pub public_parameters: Vec<PublicParameterDescriptor>,
}

/// Stable public parameter metadata derived from `preset_surface.parameters`.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicParameterDescriptor {
    pub id: String,
    pub value_type: String,
    pub default: ParameterValue,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub maps_to: PortReference,
}

/// Mutable public values owned by a loaded plugin/engine instance.
///
/// Values are keyed by stable public ID so replacement instruments and presets
/// can reconcile by public parameter identity rather than by graph structure.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InstrumentParameterState {
    pub values_by_id: BTreeMap<String, ParameterValue>,
}

/// Runtime-loaded instrument state split into immutable definition data and
/// mutable public value data.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedInstrument {
    pub definition: InstrumentDefinition,
    pub parameters: InstrumentParameterState,
}

impl InstrumentDefinition {
    pub fn from_patch(patch_doc: PatchDocument) -> Self {
        let public_parameters = patch_doc
            .preset_surface
            .parameters
            .iter()
            .map(PublicParameterDescriptor::from_parameter_target)
            .collect();

        Self {
            patch_doc,
            public_parameters,
        }
    }
}

impl PublicParameterDescriptor {
    pub fn from_parameter_target(target: &PresetParameterTargetDeclaration) -> Self {
        Self {
            id: target.name.clone(),
            value_type: format!("{:?}", target.value_type).to_lowercase(),
            default: target.default.clone(),
            min: target.min,
            max: target.max,
            maps_to: target.maps_to.clone(),
        }
    }
}

impl InstrumentParameterState {
    pub fn from_definition(definition: &InstrumentDefinition) -> Self {
        Self {
            values_by_id: definition
                .public_parameters
                .iter()
                .map(|parameter| (parameter.id.clone(), parameter.default.clone()))
                .collect(),
        }
    }

    pub fn set_value(&mut self, public_id: impl Into<String>, value: ParameterValue) {
        self.values_by_id.insert(public_id.into(), value);
    }
}

impl LoadedInstrument {
    pub fn from_patch(patch_doc: PatchDocument) -> Self {
        let definition = InstrumentDefinition::from_patch(patch_doc);
        let parameters = InstrumentParameterState::from_definition(&definition);

        Self {
            definition,
            parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::load_patch_str;

    #[test]
    fn loaded_instrument_splits_immutable_definition_from_mutable_public_values() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Stateful Instrument
instrument:
  id: dandrum.stateful
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.decay
      type: number
      default: 0.5
      min: 0
      max: 1
      maps_to: env.decay
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: env
    type: adsr
"#,
        )
        .expect("patch should parse");

        let mut loaded = LoadedInstrument::from_patch(patch);

        assert_eq!(loaded.definition.public_parameters[0].id, "tone.decay");
        assert_eq!(
            loaded.parameters.values_by_id.get("tone.decay"),
            Some(&ParameterValue::Number(0.5))
        );

        loaded
            .parameters
            .set_value("tone.decay", ParameterValue::Number(0.75));

        assert_eq!(
            loaded.definition.public_parameters[0].default,
            ParameterValue::Number(0.5)
        );
        assert_eq!(
            loaded.parameters.values_by_id.get("tone.decay"),
            Some(&ParameterValue::Number(0.75))
        );
    }
}
