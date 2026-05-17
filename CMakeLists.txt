cmake_minimum_required(VERSION 3.20)
project(cpp_rust_transpiler_support LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Runtime support library for Rust→C++ transpilation
add_library(transpiler_support INTERFACE)
target_include_directories(transpiler_support INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

# Optional: build examples
option(BUILD_EXAMPLES "Build example transpiled programs" ON)
if(BUILD_EXAMPLES)
    add_subdirectory(examples)
endif()

# Testing
include(CTest)
if(BUILD_TESTING)
    add_subdirectory(tests)
endif()
