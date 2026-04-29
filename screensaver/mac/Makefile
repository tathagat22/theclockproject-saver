BUNDLE   = ClockProjectSaver.saver
CONTENTS = $(BUNDLE)/Contents
MACOS    = $(CONTENTS)/MacOS
RESOURCES = $(CONTENTS)/Resources
BINARY   = $(MACOS)/ClockProjectSaver

.PHONY: all clean install

all: $(BUNDLE)

$(BUNDLE):
	mkdir -p $(MACOS) $(RESOURCES)
	# Compile Swift → bundle dylib
	swiftc \
		-module-name ClockProjectSaver \
		-emit-library \
		-framework Cocoa \
		-framework ScreenSaver \
		-framework WebKit \
		-target arm64-apple-macos11.0 \
		-o $(BINARY) \
		ClockProjectSaverView.swift
	# Bundle metadata
	cp Info.plist $(CONTENTS)/Info.plist
	# HTML/JS resources
	cp Resources/screensaver.html $(RESOURCES)/screensaver.html
	cp Resources/screensaver.js   $(RESOURCES)/screensaver.js
	@echo "✓ Built $(BUNDLE)"

install: all
	mkdir -p ~/Library/Screen\ Savers
	cp -r $(BUNDLE) ~/Library/Screen\ Savers/
	@echo "✓ Installed — open System Settings → Screen Saver to activate"

clean:
	rm -rf $(BUNDLE)
