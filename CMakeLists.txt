cmake_minimum_required(VERSION 3.16)
project(openclaw VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# ── 默认 Release ──────────────────────────────────────────────
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "Build type" FORCE)
endif()

# ── Release 优化 / Debug 符号 ─────────────────────────────────
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    if(MSVC)
        set(CMAKE_CXX_FLAGS_RELEASE "/O2 /GL /DNDEBUG")
    else()
        set(CMAKE_CXX_FLAGS_RELEASE "-O2 -DNDEBUG")
        # 保留最小调试信息便于崩溃报告
        add_compile_options(-g1)
    endif()
endif()

# 优先使用 Qt6，回退到 Qt5（跨平台兼容）
find_package(Qt6 QUIET COMPONENTS Core Gui Widgets Network)
if(NOT Qt6_FOUND)
    find_package(Qt5 REQUIRED COMPONENTS Core Gui Widgets Network)
    set(QT_VERSION_MAJOR 5)
    message(STATUS "Using Qt5")
else()
    set(QT_VERSION_MAJOR 6)
    message(STATUS "Using Qt6")
endif()

set(SOURCES
    src/main.cpp
    src/DeployWizard.cpp
    src/core/DeployEngine.cpp
    src/core/SystemCheck.cpp
    src/pages/WelcomePage.cpp
    src/pages/SystemCheckPage.cpp
    src/pages/InstallConfigPage.cpp
    src/pages/ServiceConfigPage.cpp
    src/pages/PlatformIntegrationPage.cpp
    src/pages/DeploymentPage.cpp
    src/pages/FinishPage.cpp
)

set(HEADERS
    src/DeployWizard.h
    src/core/DeployEngine.h
    src/core/SystemCheck.h
    src/pages/WelcomePage.h
    src/pages/SystemCheckPage.h
    src/pages/InstallConfigPage.h
    src/pages/ServiceConfigPage.h
    src/pages/PlatformIntegrationPage.h
    src/pages/DeploymentPage.h
    src/pages/FinishPage.h
)

if(QT_VERSION_MAJOR EQUAL 6)
    qt_add_executable(openclaw
        ${SOURCES}
        ${HEADERS}
        resources/resources.qrc
    )
    target_link_libraries(openclaw PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
        Qt6::Network
    )
else()
    add_executable(openclaw
        ${SOURCES}
        ${HEADERS}
        resources/resources.qrc
    )
    target_link_libraries(openclaw PRIVATE
        Qt5::Core
        Qt5::Gui
        Qt5::Widgets
        Qt5::Network
    )
endif()

target_include_directories(openclaw PRIVATE src)

# ── 编译定义：嵌入版本号 ──────────────────────────────────────
target_compile_definitions(openclaw PRIVATE
    OC_VERSION="${PROJECT_VERSION}"
    OC_APP_NAME="openclaw"
)

# ── Windows：无控制台窗口 + 管理员提权 ────────────────────────
if(WIN32)
    set_target_properties(openclaw PROPERTIES WIN32_EXECUTABLE TRUE)
    target_sources(openclaw PRIVATE resources/windows/openclaw.rc)
    if(MSVC)
        set_target_properties(openclaw PROPERTIES
            LINK_FLAGS "/MANIFESTUAC:\"level='requireAdministrator' uiAccess='false'\""
        )
    endif()
    # Qt6 需要额外链接
    if(QT_VERSION_MAJOR EQUAL 6)
        target_link_libraries(openclaw PRIVATE Qt6::EntryPointPrivate)
    endif()
endif()

# ── Linux: 去除调试符号（Release）/ RPATH ─────────────────────
if(UNIX AND NOT APPLE)
    set_target_properties(openclaw PROPERTIES
        INSTALL_RPATH "$ORIGIN/../lib"
        BUILD_WITH_INSTALL_RPATH TRUE
    )
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        # 拆分调试符号到独立 .debug 文件
        add_custom_command(TARGET openclaw POST_BUILD
            COMMAND ${CMAKE_OBJCOPY} --only-keep-debug
                $<TARGET_FILE:openclaw>
                $<TARGET_FILE:openclaw>.debug
            COMMAND ${CMAKE_STRIP} --strip-debug --strip-unneeded
                $<TARGET_FILE:openclaw>
            COMMAND ${CMAKE_OBJCOPY}
                --add-gnu-debuglink=$<TARGET_FILE:openclaw>.debug
                $<TARGET_FILE:openclaw>
            COMMENT "剥离调试信息…"
        )
    endif()
endif()

install(TARGETS openclaw
    BUNDLE  DESTINATION .
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

# ── 便捷目标 ─────────────────────────────────────────────────
add_custom_target(appimage
    COMMAND bash ${CMAKE_SOURCE_DIR}/scripts/build_appimage.sh
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    DEPENDS openclaw
    COMMENT "打包 AppImage…"
)
