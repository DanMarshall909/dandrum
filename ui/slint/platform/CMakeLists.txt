if(NOT TARGET dandrum-slint-file-uris)
    add_library(dandrum-slint-file-uris STATIC FileUris.cpp)
    add_library(Dandrum::SlintFileUris ALIAS dandrum-slint-file-uris)
    target_compile_features(dandrum-slint-file-uris PUBLIC cxx_std_20)
    target_include_directories(dandrum-slint-file-uris PUBLIC
        "${CMAKE_CURRENT_SOURCE_DIR}/.." "${CMAKE_CURRENT_SOURCE_DIR}/../../..")
endif()

if(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND NOT TARGET dandrum-slint-x11-file-drop)
    find_path(DANDRUM_SLINT_XCB_INCLUDE_DIR xcb/xcb.h)
    find_library(DANDRUM_SLINT_XCB_LIBRARY xcb)
    if(DANDRUM_SLINT_XCB_INCLUDE_DIR AND DANDRUM_SLINT_XCB_LIBRARY)
        add_library(dandrum-slint-x11-file-drop STATIC X11FileDrop.cpp)
        add_library(Dandrum::SlintX11FileDrop ALIAS dandrum-slint-x11-file-drop)
        target_include_directories(dandrum-slint-x11-file-drop PRIVATE "${DANDRUM_SLINT_XCB_INCLUDE_DIR}")
        target_link_libraries(dandrum-slint-x11-file-drop PUBLIC dandrum-slint-file-uris
            PRIVATE "${DANDRUM_SLINT_XCB_LIBRARY}")
        target_compile_definitions(dandrum-slint-x11-file-drop PUBLIC
            DANDRUM_SLINT_HAVE_X11_FILE_DROP DANDRUM_HAVE_X11_FILE_DROP)
    endif()
endif()
