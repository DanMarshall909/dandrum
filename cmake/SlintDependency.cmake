include_guard(GLOBAL)
include(FetchContent)

# Both native UI experiments use the same pinned software renderer.
function(dandrum_fetch_slint)
    set(BUILD_SHARED_LIBS OFF)
    set(CMAKE_POSITION_INDEPENDENT_CODE ON)
    set(SLINT_FEATURE_BACKEND_WINIT OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_BACKEND_WINIT_X11 OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_BACKEND_WINIT_WAYLAND OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_BACKEND_QT OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_BACKEND_LINUXKMS OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_RENDERER_FEMTOVG OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_RENDERER_SKIA OFF CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_RENDERER_SOFTWARE ON CACHE BOOL "" FORCE)
    set(SLINT_FEATURE_INTERPRETER OFF CACHE BOOL "" FORCE)
    FetchContent_Declare(Slint
        URL https://github.com/slint-ui/slint/archive/372cf0ee5577c3dfec309a45e7b778ba4e81b734.tar.gz
        SOURCE_SUBDIR api/cpp DOWNLOAD_EXTRACT_TIMESTAMP TRUE)
    FetchContent_MakeAvailable(Slint)
    if(TARGET slint-compiler AND COMMAND corrosion_set_env_vars)
        # jemalloc appends outer Make's compact flags after Cargo's jobserver flags.
        # A bare "s" then becomes a target name. Let Cargo supply its own flags.
        corrosion_set_env_vars(slint-compiler "MAKEFLAGS=" "MFLAGS=")
    endif()
endfunction()
dandrum_fetch_slint()
