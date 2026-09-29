if(NOT EXISTS "${RUST_ENGINE_LIB}")
    message(FATAL_ERROR "Rust engine archive is missing: ${RUST_ENGINE_LIB}")
endif()

execute_process(
    COMMAND "${NM}" -g --defined-only "${RUST_ENGINE_LIB}"
    RESULT_VARIABLE nm_status
    OUTPUT_VARIABLE symbols
    ERROR_VARIABLE nm_error)
if(NOT nm_status EQUAL 0)
    message(FATAL_ERROR "Could not inspect Rust engine symbols: ${nm_error}")
endif()

if(NOT symbols MATCHES " [Tt] dandrum_kernel_prepare_file")
    message(FATAL_ERROR "Kernel preparation export is missing from the Rust engine archive")
endif()

string(REGEX MATCHALL " [Tt] dandrum_engine_[A-Za-z0-9_]+" retired_symbols "${symbols}")
if(retired_symbols)
    message(FATAL_ERROR "Retired engine FFI exports remain: ${retired_symbols}")
endif()

execute_process(
    COMMAND "${NM}" -C --defined-only "${RUST_ENGINE_LIB}"
    RESULT_VARIABLE demangled_nm_status
    OUTPUT_VARIABLE demangled_symbols
    ERROR_VARIABLE demangled_nm_error)
if(NOT demangled_nm_status EQUAL 0)
    message(FATAL_ERROR "Could not inspect demangled Rust symbols: ${demangled_nm_error}")
endif()

string(REGEX MATCH
    "dandrum_engine::(voice_allocator::|patch::(PatchDocument|VoiceAllocation|PresetSurfaceDeclaration|apply_preset|load_patch_file|load_patch_str|validate_patch_schema|resolve_module_parameters)|graph::(ExecutionScope|VoiceToGlobalDirectRouting|ModuleNode::params)|compiled_patch::legacy_node_data|graph_processor::render_plan::AudioOutputBinding|graph_processor::realtime_graph_processor::(render_chunk|render_mono_global_arena|render_polyphonic_from_plan))"
    retired_path_symbol "${demangled_symbols}")
if(retired_path_symbol)
    message(FATAL_ERROR "Retired production path remains in the Rust archive: ${retired_path_symbol}")
endif()
