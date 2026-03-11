cmake_minimum_required(VERSION 3.16)
project(openclaw VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

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

# Windows: 无控制台窗口 + 管理员提权
if(WIN32)
    set_target_properties(openclaw PROPERTIES WIN32_EXECUTABLE TRUE)
    if(MSVC)
        target_sources(openclaw PRIVATE resources/windows/openclaw.rc)
        set_target_properties(openclaw PROPERTIES
            LINK_FLAGS "/MANIFESTUAC:\"level='requireAdministrator' uiAccess='false'\""
        )
    elseif(MINGW)
        target_sources(openclaw PRIVATE resources/windows/openclaw.rc)
    endif()
endif()

# Linux/macOS: RPATH
if(UNIX AND NOT APPLE)
    set_target_properties(openclaw PROPERTIES
        INSTALL_RPATH "$ORIGIN/../lib"
        BUILD_WITH_INSTALL_RPATH TRUE
    )
endif()

install(TARGETS openclaw
    BUNDLE DESTINATION .
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
