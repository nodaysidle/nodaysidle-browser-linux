QT += core gui qml quick quickcontrols2 webenginequick network

CONFIG += c++17 release
TARGET = nodaysidle-browser
TEMPLATE = app

HEADERS += \
    src/backend.h \
    src/theme.h \
    src/adblock.h

SOURCES += \
    src/main.cpp \
    src/backend.cpp \
    src/theme.cpp

RESOURCES += src/resources.qrc
