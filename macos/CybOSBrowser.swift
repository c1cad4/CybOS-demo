import AppKit
import WebKit

final class Browser: NSObject, NSApplicationDelegate, WKNavigationDelegate, WKUIDelegate {
    var window: NSWindow!
    var web: WKWebView!
    let address = NSTextField()
    let status = NSTextField(labelWithString: "Приватная сессия · сайты видят ваш сетевой адрес")
    var back: NSButton!
    var forward: NSButton!

    func applicationDidFinishLaunching(_ notification: Notification) {
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .nonPersistent()
        web = WKWebView(frame: .zero, configuration: config)
        web.navigationDelegate = self
        web.uiDelegate = self
        web.allowsBackForwardNavigationGestures = true
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1100, height: 760), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
        window.title = "CybBrowser"
        window.minSize = NSSize(width: 700, height: 500)
        window.isReleasedWhenClosed = false
        let root = window.contentView!
        let toolbar = NSStackView()
        toolbar.orientation = .horizontal
        toolbar.spacing = 8
        back = NSButton(title: "←", target: self, action: #selector(goBack))
        forward = NSButton(title: "→", target: self, action: #selector(goForward))
        let reload = NSButton(title: "Обновить", target: self, action: #selector(refresh))
        let stop = NSButton(title: "Стоп", target: self, action: #selector(stopLoading))
        address.placeholderString = "Поиск или адрес сайта"
        address.target = self
        address.action = #selector(navigate)
        toolbar.addArrangedSubview(back)
        toolbar.addArrangedSubview(forward)
        toolbar.addArrangedSubview(reload)
        toolbar.addArrangedSubview(stop)
        toolbar.addArrangedSubview(address)
        toolbar.addArrangedSubview(NSButton(title: "Открыть", target: self, action: #selector(navigate)))
        for view in [toolbar, web!, status] as [NSView] {
            view.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(view)
        }
        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: root.topAnchor, constant: 12), toolbar.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 12), toolbar.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -12),
            web.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 12), web.leadingAnchor.constraint(equalTo: root.leadingAnchor), web.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            status.topAnchor.constraint(equalTo: web.bottomAnchor, constant: 6), status.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 12), status.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -12), status.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -8)
        ])
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        address.stringValue = CommandLine.arguments.dropFirst().first ?? "https://duckduckgo.com"
        navigate()
    }
    @objc func navigate() {
        let text = address.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        var target = text
        if !text.contains("://") {
            if !text.contains(" ") && text.contains(".") { target = "https://" + text }
            else {
                var parts = URLComponents(string: "https://duckduckgo.com/")!
                parts.queryItems = [URLQueryItem(name: "q", value: text)]
                target = parts.url!.absoluteString
            }
        }
        guard let url = URL(string: target), ["https", "http"].contains(url.scheme?.lowercased() ?? ""), url.host != nil else {
            status.stringValue = "Введите поисковый запрос или адрес HTTP/HTTPS"
            return
        }
        web.load(URLRequest(url: url))
    }
    @objc func goBack() { web.goBack() }
    @objc func goForward() { web.goForward() }
    @objc func refresh() { web.reload() }
    @objc func stopLoading() { web.stopLoading() }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        address.stringValue = webView.url?.absoluteString ?? address.stringValue
        window.title = "CybBrowser · " + (webView.title ?? "Страница")
        back.isEnabled = webView.canGoBack
        forward.isEnabled = webView.canGoForward
        status.stringValue = "Приватная сессия · сайты видят ваш сетевой адрес"
    }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { status.stringValue = error.localizedDescription }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { status.stringValue = error.localizedDescription }
    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        let scheme = navigationAction.request.url?.scheme?.lowercased() ?? ""
        decisionHandler(["http", "https", "about"].contains(scheme) ? .allow : .cancel)
    }
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration, for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        if navigationAction.targetFrame == nil { webView.load(navigationAction.request) }
        return nil
    }
    func webView(_ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping () -> Void) {
        let alert = NSAlert(); alert.messageText = "Сообщение сайта"; alert.informativeText = message; alert.runModal(); completionHandler()
    }
    func webView(_ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping (Bool) -> Void) {
        let alert = NSAlert(); alert.messageText = "Подтверждение сайта"; alert.informativeText = message; alert.addButton(withTitle: "Продолжить"); alert.addButton(withTitle: "Отмена"); completionHandler(alert.runModal() == .alertFirstButtonReturn)
    }
}
let app = NSApplication.shared
let delegate = Browser()
app.setActivationPolicy(.regular)
app.delegate = delegate
app.run()
