## MODIFIED Requirements

### Requirement: Complete instrument editing
The shell SHALL implement selector/candidate operations, voice template/modules/policy/envelopes, modulation sources/routes, macros/MIDI learn, routing/sends/graph and insert/FX operations. It SHALL present the default amplitude envelope as a replaceable assignment with advanced Replace modulator, Detach assignment and Restore default actions. In native mode, the shell SHALL present only actions supported by the connected adapter.

#### Scenario: Layers, voices and modulation
- **WHEN** the user edits any supported selector, candidate, voice, envelope, modulator, macro or route control or context-menu action
- **THEN** the current model, previews and inspector reflect its actual adapter operation, with undo, assignment safety for the default amplitude route and no inert enabled menu items.

#### Scenario: Amplitude assignment actions
- **WHEN** the user replaces, detaches or restores the amplitude assignment through the advanced controls
- **THEN** the Voice and Modulation pages SHALL identify the accepted source or detached state, preserve the ordinary amplitude-envelope panel when the default is assigned, and make the action undoable

#### Scenario: Routing and effects
- **WHEN** the user changes inherited outputs/sends or adds/moves/bypasses a processor
- **THEN** routing tables/graph and bus/chain views reflect the accepted operation and undo restores it.
