import ScreenSaver
import WebKit

final class ClockProjectView: ScreenSaverView, WKNavigationDelegate {

    private var webView: WKWebView!

    // MARK: - Init

    override init?(frame: NSRect, isPreview: Bool) {
        super.init(frame: frame, isPreview: isPreview)
        setup()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        let config = WKWebViewConfiguration()
        let cacheDir = cacheDirectory()

        // Inject cache path + saved styles into JS before page loads
        let js = buildInjectedJS(cacheDir: cacheDir)
        config.userContentController.addUserScript(
            WKUserScript(source: js, injectionTime: .atDocumentStart, forMainFrameOnly: true)
        )

        webView = WKWebView(frame: bounds, configuration: config)
        webView.autoresizingMask = [.width, .height]
        webView.navigationDelegate = self
        webView.setValue(false, forKey: "drawsBackground")
        addSubview(webView)

        guard let htmlURL = Bundle(for: type(of: self))
            .url(forResource: "screensaver", withExtension: "html", subdirectory: "Resources")
        else { return }

        // Grant read access to the entire cache dir so file:// image URLs work
        webView.loadFileURL(htmlURL, allowingReadAccessTo: cacheDir)
    }

    // MARK: - ScreenSaverView

    override func startAnimation() {
        super.startAnimation()
        webView?.evaluateJavaScript("window.startClock && window.startClock()", completionHandler: nil)
    }

    override func stopAnimation() {
        super.stopAnimation()
        webView?.evaluateJavaScript("window.stopClock && window.stopClock()", completionHandler: nil)
    }

    override func draw(_ rect: NSRect) {
        NSColor.black.setFill()
        rect.fill()
    }

    override var hasConfigureSheet: Bool { false }
    override var configureSheet: NSWindow? { nil }

    // MARK: - Helpers

    private func cacheDirectory() -> URL {
        FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("theclockproject-saver")
    }

    private func buildInjectedJS(cacheDir: URL) -> String {
        // Read saved styles from settings.json
        let settingsURL = cacheDir.appendingPathComponent("settings.json")
        var stylesArray = "[\"clock_face\"]"
        if let data = try? Data(contentsOf: settingsURL),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let styles = json["styles"] as? [String],
           !styles.isEmpty {
            let quoted = styles.map { "\"\($0)\"" }.joined(separator: ", ")
            stylesArray = "[\(quoted)]"
        }

        return """
        window.CACHE_DIR = \"\(cacheDir.path)\";
        window.CLOCK_STYLES = \(stylesArray);
        """
    }
}
