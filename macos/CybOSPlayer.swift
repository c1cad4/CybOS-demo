import AppKit
import AVKit
import AVFoundation

final class PlayerDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var player: AVPlayer?
    var observer: NSKeyValueObservation?
    let status = NSTextField(labelWithString: "Ожидание частей фильма…")
    var mediaURL: URL?
    func applicationDidFinishLaunching(_ notification: Notification) {
        let menu = NSMenu()
        let menuItem = NSMenuItem(); menu.addItem(menuItem)
        let submenu = NSMenu(); submenu.addItem(withTitle: "Закрыть CybPlayer", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        menuItem.submenu = submenu; NSApp.mainMenu = menu
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1100, height: 700), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = "CybPlayer"
        window.isReleasedWhenClosed = false
        window.backgroundColor = NSColor(calibratedRed: 0.02, green: 0.05, blue: 0.04, alpha: 1)
        let root = NSView(); window.contentView = root
        let video = AVPlayerView(); video.controlsStyle = .floating
        video.showsFullScreenToggleButton = true
        let vlc = NSButton(title: "Открыть в VLC", target: self, action: #selector(openVLC))
        vlc.isEnabled = NSWorkspace.shared.urlForApplication(withBundleIdentifier: "org.videolan.vlc") != nil
        status.textColor = .white; status.lineBreakMode = .byTruncatingTail
        for view in [video, status, vlc] as [NSView] { root.addSubview(view); view.translatesAutoresizingMaskIntoConstraints = false }
        NSLayoutConstraint.activate([
            video.topAnchor.constraint(equalTo: root.topAnchor), video.leadingAnchor.constraint(equalTo: root.leadingAnchor), video.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            video.bottomAnchor.constraint(equalTo: status.topAnchor, constant: -10),
            status.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 12), status.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -12),
            status.trailingAnchor.constraint(equalTo: vlc.leadingAnchor, constant: -12),
            vlc.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -12), vlc.centerYAnchor.constraint(equalTo: status.centerYAnchor)
        ])
        window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        guard let raw = CommandLine.arguments.dropFirst().first, let url = URL(string: raw), url.scheme == "http", url.host == "127.0.0.1", url.port != nil, url.path.hasPrefix("/watch/"), url.query == nil else {
            status.stringValue = "Запусти просмотр из библиотеки CybLex"; return
        }
        mediaURL = url
        let item = AVPlayerItem(url: url)
        player = AVPlayer(playerItem: item); video.player = player
        observer = item.observe(\.status, options: [.initial, .new]) { [weak self] item, _ in
            DispatchQueue.main.async {
                switch item.status {
                case .readyToPlay: self?.status.stringValue = "CybLex · просмотр во время загрузки"; self?.player?.play()
                case .failed: self?.status.stringValue = "Не удалось воспроизвести: проверь загрузку. Для неподдерживаемых кодеков используй VLC."
                default: self?.status.stringValue = "Ожидание частей фильма…"
                }
            }
        }
    }
    @objc func openVLC() {
        guard let url = mediaURL else { return }
        player?.pause()
        let task = Process(); task.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        task.arguments = ["-n", "-a", "VLC", "--args", url.absoluteString]
        do { try task.run() } catch { status.stringValue = "Не удалось открыть VLC" }
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ notification: Notification) { player?.pause(); observer?.invalidate() }
}
let app = NSApplication.shared
let delegate = PlayerDelegate()
app.setActivationPolicy(.regular); app.delegate = delegate; app.run()
