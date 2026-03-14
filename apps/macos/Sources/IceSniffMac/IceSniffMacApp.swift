import SwiftUI
import UniformTypeIdentifiers
import AppKit

struct WindowChromeConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        DispatchQueue.main.async {
            guard let window = view.window else { return }
            configure(window)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            guard let window = nsView.window else { return }
            configure(window)
        }
    }

    private func configure(_ window: NSWindow) {
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.toolbarStyle = .unified
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = false
        window.backgroundColor = .clear
        window.toolbar?.showsBaselineSeparator = false
        window.makeKeyAndOrderFront(nil)
        NSApp.setActivationPolicy(.regular)
        if let iconImage = bundledAppIconImage() {
            NSApp.applicationIconImage = iconImage
        }
        NSApp.activate(ignoringOtherApps: true)
    }
}

@main
struct IceSniffMacApp: App {
    @StateObject private var model = AppModel()
    @State private var presentingFileImporter = false

    private func presentSavePanel(scope: CaptureSaveScope) {
        let panel = NSSavePanel()
        panel.canCreateDirectories = true
        panel.isExtensionHidden = false
        panel.nameFieldStringValue = "capture"
        panel.allowedContentTypes = [
            UTType(filenameExtension: "pcap") ?? .data
        ]

        guard panel.runModal() == .OK, let url = panel.url else {
            return
        }

        model.saveCapture(to: url.path, scope: scope)
    }

    private func presentSaveFlow() {
        guard model.hasActiveFilter else {
            presentSavePanel(scope: .wholeCapture)
            return
        }

        let alert = NSAlert()
        alert.messageText = "Save Capture"
        alert.informativeText = "A filter is currently applied. Do you want to save only the filtered packets, or the whole capture?"
        alert.alertStyle = .informational
        alert.addButton(withTitle: "Filtered Only")
        alert.addButton(withTitle: "Whole Capture")
        alert.addButton(withTitle: "Cancel")

        switch alert.runModal() {
        case .alertFirstButtonReturn:
            presentSavePanel(scope: .filteredOnly)
        case .alertSecondButtonReturn:
            presentSavePanel(scope: .wholeCapture)
        default:
            return
        }
    }

    var body: some Scene {
        WindowGroup {
            ZStack {
                LiquidBackdrop(theme: model.appTheme)
                    .ignoresSafeArea()

                HStack(spacing: 0) {
                    SidebarView(
                        model: model,
                        openCaptureAction: {
                            presentingFileImporter = true
                        }
                    )

                    DetailView(
                        model: model,
                        saveCaptureAction: {
                            presentSaveFlow()
                        },
                        openCaptureAction: {
                            presentingFileImporter = true
                        }
                    )
                }
            }
            .frame(minWidth: 1240, minHeight: 820)
            .preferredColorScheme(model.darkMode ? .dark : .light)
            .environment(\.font, appFont(model.fontChoice, .body))
            .toolbar {
                ToolbarItem(placement: .navigation) {
                    Button {
                        withAnimation(.spring(duration: 0.35, bounce: 0.22)) {
                            model.toggleSidebar()
                        }
                    } label: {
                        Label(
                            model.sidebarCollapsed ? "Show Sidebar" : "Hide Sidebar",
                            systemImage: model.sidebarCollapsed ? "sidebar.right" : "sidebar.left"
                        )
                    }
                }
            }
            .fileImporter(
                isPresented: $presentingFileImporter,
                allowedContentTypes: [.data],
                allowsMultipleSelection: false
            ) { result in
                switch result {
                case let .success(urls):
                    guard let selected = urls.first else { return }
                    model.setCapturePath(selected.path)
                    model.refreshAll()
                case let .failure(error):
                    model.setStatus(message: "File picker failed: \(error.localizedDescription)")
                }
            }
            .background(WindowChromeConfigurator().allowsHitTesting(false))
        }
        .windowToolbarStyle(.unified(showsTitle: false))
    }
}
