import ScreenSaver
import AppKit

// Minimal .saver bundle that launches the Tauri app binary and takes it fullscreen.
// Build: xcodebuild -target ClockProjectSaver (see Makefile in this dir)
// Install: copy ClockProjectSaver.saver to ~/Library/Screen Savers/

class ClockProjectView: ScreenSaverView {
    private var appProcess: Process?

    override init?(frame: NSRect, isPreview: Bool) {
        super.init(frame: frame, isPreview: isPreview)
        // Nothing to set up in preview — just show a black view
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
    }

    override func startAnimation() {
        super.startAnimation()
        guard appProcess == nil else { return }

        // The Tauri binary is bundled inside the .saver under Contents/Resources/
        guard let binaryURL = Bundle(for: type(of: self))
            .url(forResource: "theclockproject-saver", withExtension: nil) else {
            return
        }

        let process = Process()
        process.executableURL = binaryURL
        process.arguments = ["--screensaver"] // Tauri can react to this flag
        try? process.run()
        appProcess = process
    }

    override func stopAnimation() {
        super.stopAnimation()
        appProcess?.terminate()
        appProcess = nil
    }

    override func draw(_ rect: NSRect) {
        NSColor.black.setFill()
        rect.fill()
    }

    override var hasConfigureSheet: Bool { false }
    override var configureSheet: NSWindow? { nil }
}
