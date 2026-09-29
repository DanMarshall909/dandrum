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
