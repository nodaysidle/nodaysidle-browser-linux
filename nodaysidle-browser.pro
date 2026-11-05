QT += core gui qml quick quickcontrols2 webenginequick network sql

LIBS += -lcrypto

CONFIG += c++17 release
TARGET = nodaysidle-browser
TEMPLATE = app

HEADERS += \
    src/backend.h \
    src/theme.h \
    src/adblock.h \
    src/vault.h \
    src/importer.h

SOURCES += \
    src/main.cpp \
    src/backend.cpp \
    src/theme.cpp \
    src/vault.cpp \
    src/importer.cpp

RESOURCES += src/resources.qrc
