cmake_minimum_required(VERSION 3.22)

if (NOT DEFINED SOURCE_ROOT)
    message(FATAL_ERROR "SOURCE_ROOT must point to the repository root")
endif()

function(read_source_relative out_var relative_path)
    file(READ "${SOURCE_ROOT}/${relative_path}" source_text)
    set(${out_var} "${source_text}" PARENT_SCOPE)
endfunction()

function(extract_function_body out_var source_text signature)
    string(FIND "${source_text}" "${signature}" signature_pos)
    if (signature_pos EQUAL -1)
        message(FATAL_ERROR "Could not find function signature: ${signature}")
    endif()

    string(SUBSTRING "${source_text}" ${signature_pos} -1 after_signature)
    string(FIND "${after_signature}" "{" open_offset)
    if (open_offset EQUAL -1)
        message(FATAL_ERROR "Could not find function body for: ${signature}")
    endif()

    string(SUBSTRING "${after_signature}" ${open_offset} -1 body_and_rest)
    string(FIND "${body_and_rest}" "\n}" close_offset)
    if (close_offset EQUAL -1)
        message(FATAL_ERROR "Could not find end of function body for: ${signature}")
    endif()

    string(SUBSTRING "${body_and_rest}" 0 ${close_offset} body)
    set(${out_var} "${body}" PARENT_SCOPE)
endfunction()

read_source_relative(rust_engine_source "src/juce-wrapper/RustEngineSource.cpp")
extract_function_body(
    get_next_audio_block_body
    "${rust_engine_source}"
    "void RustEngineSource::getNextAudioBlock")

if (get_next_audio_block_body MATCHES "ScopedLock|CriticalSection|std::cout|std::cerr")
    message(FATAL_ERROR "RustEngineSource::getNextAudioBlock contains callback-unsafe locking or console IO")
endif()

extract_function_body(
    note_on_body
    "${rust_engine_source}"
    "bool RustEngineSource::noteOn")

if (note_on_body MATCHES "ScopedLock|CriticalSection|dandrum_engine_note_on")
    message(FATAL_ERROR "RustEngineSource::noteOn must enqueue without locking or directly mutating the engine")
endif()

extract_function_body(
    note_off_body
    "${rust_engine_source}"
    "bool RustEngineSource::noteOff")

if (note_off_body MATCHES "ScopedLock|CriticalSection|dandrum_engine_note_off")
    message(FATAL_ERROR "RustEngineSource::noteOff must enqueue without locking or directly mutating the engine")
endif()

read_source_relative(midi_to_rust_engine "src/juce-wrapper/MidiToRustEngine.cpp")
extract_function_body(
    handle_incoming_midi_body
    "${midi_to_rust_engine}"
    "void MidiToRustEngine::handleIncomingMidiMessage")

if (handle_incoming_midi_body MATCHES "std::cout|std::cerr")
    message(FATAL_ERROR "MidiToRustEngine::handleIncomingMidiMessage contains callback-unsafe console IO")
endif()
