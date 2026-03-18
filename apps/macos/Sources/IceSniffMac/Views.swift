import SwiftUI
import AppKit

private struct ThemePalette {
    let accent: Color
    let backgroundGradient: [Color]
    let glowA: Color
    let glowB: Color
    let glowC: Color
}

private func palette(for theme: AppTheme) -> ThemePalette {
    switch theme {
    case .defaultDark:
        return ThemePalette(
            accent: Color(red: 0.33, green: 0.72, blue: 1.0),
            backgroundGradient: [Color(red: 0.02, green: 0.03, blue: 0.05), Color(red: 0.06, green: 0.08, blue: 0.12)],
            glowA: Color(red: 0.24, green: 0.56, blue: 0.96).opacity(0.22),
            glowB: Color.pink.opacity(0.16),
            glowC: Color.cyan.opacity(0.14)
        )
    case .defaultLight:
        return ThemePalette(
            accent: Color(red: 0.08, green: 0.47, blue: 0.9),
            backgroundGradient: [Color(red: 0.92, green: 0.95, blue: 0.99), Color(red: 0.85, green: 0.9, blue: 0.98)],
            glowA: Color(red: 0.28, green: 0.63, blue: 0.98).opacity(0.2),
            glowB: Color.pink.opacity(0.13),
            glowC: Color.cyan.opacity(0.1)
        )
    case .ocean:
        return ThemePalette(
            accent: Color(red: 0.18, green: 0.78, blue: 0.78),
            backgroundGradient: [Color(red: 0.01, green: 0.08, blue: 0.13), Color(red: 0.05, green: 0.16, blue: 0.23)],
            glowA: Color(red: 0.17, green: 0.78, blue: 0.85).opacity(0.26),
            glowB: Color(red: 0.04, green: 0.45, blue: 0.68).opacity(0.22),
            glowC: Color(red: 0.36, green: 0.9, blue: 0.76).opacity(0.18)
        )
    case .ember:
        return ThemePalette(
            accent: Color(red: 0.98, green: 0.48, blue: 0.28),
            backgroundGradient: [Color(red: 0.09, green: 0.03, blue: 0.04), Color(red: 0.2, green: 0.08, blue: 0.08)],
            glowA: Color(red: 0.98, green: 0.45, blue: 0.24).opacity(0.24),
            glowB: Color(red: 0.95, green: 0.22, blue: 0.38).opacity(0.18),
            glowC: Color(red: 1.0, green: 0.72, blue: 0.3).opacity(0.14)
        )
    case .forest:
        return ThemePalette(
            accent: Color(red: 0.42, green: 0.78, blue: 0.46),
            backgroundGradient: [Color(red: 0.03, green: 0.07, blue: 0.04), Color(red: 0.08, green: 0.14, blue: 0.1)],
            glowA: Color(red: 0.34, green: 0.78, blue: 0.42).opacity(0.23),
            glowB: Color(red: 0.12, green: 0.47, blue: 0.32).opacity(0.18),
            glowC: Color(red: 0.72, green: 0.88, blue: 0.44).opacity(0.12)
        )
    }
}

private func fontPointSize(for textStyle: Font.TextStyle) -> CGFloat {
    switch textStyle {
    case .largeTitle: return 34
    case .title: return 28
    case .title2: return 22
    case .title3: return 20
    case .headline: return 13
    case .subheadline: return 12
    case .body: return 13
    case .callout: return 12
    case .caption: return 11
    case .caption2: return 10
    case .footnote: return 11
    @unknown default: return 13
    }
}

func appFont(_ choice: AppFontChoice, _ textStyle: Font.TextStyle, weight: Font.Weight = .regular, scale: CGFloat = 1.0) -> Font {
    let design: Font.Design = switch choice {
    case .system: .default
    case .rounded: .rounded
    case .serif: .serif
    case .monospaced: .monospaced
    }
    return .system(size: fontPointSize(for: textStyle) * scale, weight: weight, design: design)
}

private func appNSFont(_ choice: AppFontChoice, size: CGFloat, weight: NSFont.Weight = .regular, scale: CGFloat = 1.0) -> NSFont {
    let scaledSize = size * scale
    switch choice {
    case .system:
        return .systemFont(ofSize: scaledSize, weight: weight)
    case .rounded:
        let descriptor = NSFont.systemFont(ofSize: scaledSize, weight: weight).fontDescriptor.withDesign(.rounded)
        return descriptor.flatMap { NSFont(descriptor: $0, size: scaledSize) } ?? .systemFont(ofSize: scaledSize, weight: weight)
    case .serif:
        return .userFont(ofSize: scaledSize) ?? .systemFont(ofSize: scaledSize, weight: weight)
    case .monospaced:
        return .monospacedSystemFont(ofSize: scaledSize, weight: weight)
    }
}

private func accentTint(_ theme: AppTheme) -> Color {
    palette(for: theme).accent
}

private func cardStroke(_ darkMode: Bool, theme: AppTheme) -> LinearGradient {
    LinearGradient(
        colors: [
            accentTint(theme).opacity(darkMode ? 0.22 : 0.2),
            Color.white.opacity(darkMode ? 0.08 : 0.26)
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

private func copyToPasteboard(_ value: String) {
    let pasteboard = NSPasteboard.general
    pasteboard.clearContents()
    pasteboard.setString(value, forType: .string)
}

func bundledAppIconImage() -> NSImage? {
    guard let iconURL = Bundle.module.url(forResource: "icon-light", withExtension: "png"),
          let image = NSImage(contentsOf: iconURL) else {
        return nil
    }

    return paddedAppIconImage(from: image)
}

private func paddedAppIconImage(from image: NSImage, insetRatio: CGFloat = 0.16) -> NSImage? {
    let canvasSize = NSSize(width: 1024, height: 1024)
    let paddedImage = NSImage(size: canvasSize)

    paddedImage.lockFocus()
    NSColor.clear.set()
    NSRect(origin: .zero, size: canvasSize).fill()

    let inset = canvasSize.width * insetRatio
    let destinationRect = NSRect(
        x: inset,
        y: inset,
        width: canvasSize.width - (inset * 2),
        height: canvasSize.height - (inset * 2)
    )

    image.draw(
        in: destinationRect,
        from: NSRect(origin: .zero, size: image.size),
        operation: .sourceOver,
        fraction: 1
    )
    paddedImage.unlockFocus()
    paddedImage.size = NSSize(width: 512, height: 512)

    return paddedImage
}

struct LiquidBackdrop: View {
    let theme: AppTheme

    var body: some View {
        let colors = palette(for: theme)

        ZStack {
            LinearGradient(
                colors: colors.backgroundGradient,
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            Circle()
                .fill(colors.glowA)
                .frame(width: 520, height: 520)
                .blur(radius: 80)
                .offset(x: -380, y: -260)

            Circle()
                .fill(colors.glowB)
                .frame(width: 440, height: 440)
                .blur(radius: 90)
                .offset(x: 420, y: -220)

            Circle()
                .fill(colors.glowC)
                .frame(width: 640, height: 640)
                .blur(radius: 110)
                .offset(x: 240, y: 340)
        }
    }
}

struct LiquidCard<Content: View>: View {
    let theme: AppTheme
    var cornerRadius: CGFloat = 24
    var padding: CGFloat = 14
    @ViewBuilder var content: () -> Content

    private var darkMode: Bool { theme.isDark }

    var body: some View {
        content()
            .padding(padding)
            .background {
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(.ultraThinMaterial)
                    .allowsHitTesting(false)
                    .overlay {
                        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                            .fill(accentTint(theme).opacity(darkMode ? 0.1 : 0.05))
                            .blendMode(.plusLighter)
                            .allowsHitTesting(false)
                    }
                    .overlay {
                        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                            .stroke(cardStroke(darkMode, theme: theme), lineWidth: 1)
                            .allowsHitTesting(false)
                    }
                    .shadow(color: Color.black.opacity(darkMode ? 0.32 : 0.1), radius: 20, x: 0, y: 16)
            }
    }
}

struct NativeTextField: NSViewRepresentable {
    let placeholder: String
    @Binding var text: String
    var font: NSFont

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    func makeNSView(context: Context) -> NSTextField {
        let textField = NSTextField()
        textField.placeholderString = placeholder
        textField.delegate = context.coordinator
        textField.isBezeled = true
        textField.isBordered = true
        textField.isEditable = true
        textField.isSelectable = true
        textField.drawsBackground = true
        textField.focusRingType = .default
        textField.bezelStyle = .roundedBezel
        textField.font = font
        textField.stringValue = text
        return textField
    }

    func updateNSView(_ nsView: NSTextField, context: Context) {
        if nsView.stringValue != text {
            nsView.stringValue = text
        }
        nsView.placeholderString = placeholder
        nsView.font = font
    }

    final class Coordinator: NSObject, NSTextFieldDelegate {
        @Binding var text: String

        init(text: Binding<String>) {
            _text = text
        }

        func controlTextDidChange(_ notification: Notification) {
            guard let textField = notification.object as? NSTextField else { return }
            text = textField.stringValue
        }
    }
}

struct ChatComposerTextView: NSViewRepresentable {
    @Binding var text: String
    var font: NSFont
    let onSubmit: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text, onSubmit: onSubmit)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSScrollView()
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true

        let textView = SendOnEnterTextView()
        textView.delegate = context.coordinator
        textView.isRichText = false
        textView.importsGraphics = false
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isContinuousSpellCheckingEnabled = false
        textView.backgroundColor = .clear
        textView.drawsBackground = false
        textView.font = font
        textView.string = text
        textView.textContainerInset = NSSize(width: 0, height: 8)
        textView.sendHandler = onSubmit

        scrollView.documentView = textView
        return scrollView
    }

    func updateNSView(_ nsView: NSScrollView, context: Context) {
        guard let textView = nsView.documentView as? SendOnEnterTextView else { return }
        if textView.string != text {
            textView.string = text
        }
        textView.font = font
        textView.sendHandler = onSubmit
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        @Binding var text: String
        let onSubmit: () -> Void

        init(text: Binding<String>, onSubmit: @escaping () -> Void) {
            _text = text
            self.onSubmit = onSubmit
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            text = textView.string
        }
    }
}

final class SendOnEnterTextView: NSTextView {
    var sendHandler: (() -> Void)?

    override func keyDown(with event: NSEvent) {
        let isReturn = event.keyCode == 36 || event.keyCode == 76
        let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let shouldInsertNewline = modifiers.contains(.shift) || modifiers.contains(.option)

        if isReturn && !shouldInsertNewline {
            sendHandler?()
            return
        }

        super.keyDown(with: event)
    }
}

struct SidebarView: View {
    @ObservedObject var model: AppModel
    let openCaptureAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 10) {
                Group {
                    if let nsImage = bundledAppIconImage() {
                        Image(nsImage: nsImage)
                            .resizable()
                            .scaledToFit()
                    } else {
                        Image(systemName: "network")
                            .resizable()
                            .scaledToFit()
                            .padding(7)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(width: model.sidebarCollapsed ? 60 : 38, height: model.sidebarCollapsed ? 60 : 38)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))

                if !model.sidebarCollapsed {
                    VStack(alignment: .leading, spacing: 1) {
                        Text("IceSniff")
                            .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                        Text("Capture Browser")
                            .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                            .foregroundStyle(.secondary)
                    }
                }

                if !model.sidebarCollapsed {
                    Spacer(minLength: 0)
                }
            }
            .frame(maxWidth: .infinity, alignment: model.sidebarCollapsed ? .center : .leading)
            .padding(.bottom, 6)
            .padding(.top, model.sidebarCollapsed ? 44 : 50)


            VStack(alignment: .leading, spacing: 6) {
                ForEach(AppSection.primarySections) { section in
                    SidebarButton(
                        title: section.title,
                        icon: section.iconSystemName,
                        collapsed: model.sidebarCollapsed,
                        isSelected: model.selectedSection == section,
                        theme: model.appTheme,
                        fontChoice: model.fontChoice,
                        circularWhenCollapsed: false
                    ) {
                        model.selectedSection = section
                    }
                }
            }

            Spacer()


            VStack(alignment: .leading, spacing: 6) {
                if !model.sidebarCollapsed {
                    Button {
                        openCaptureAction()
                    } label: {
                        HStack(spacing: 8) {
                            Image(systemName: "folder.fill.badge.plus")
                            Text("Open Capture")
                                .font(appFont(model.fontChoice, .subheadline, weight: .semibold, scale: model.fontScale))
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(Color.white)
                    .background(
                        RoundedRectangle(cornerRadius: 11, style: .continuous)
                            .fill(
                                LinearGradient(
                                    colors: [accentTint(model.appTheme), accentTint(model.appTheme).opacity(0.78)],
                                    startPoint: .topLeading,
                                    endPoint: .bottomTrailing
                                )
                            )
                    )
                    .overlay {
                        RoundedRectangle(cornerRadius: 11, style: .continuous)
                            .stroke(Color.white.opacity(0.2), lineWidth: 1)
                    }
                    .padding(.bottom, 6)
                }

                SidebarButton(
                    title: AppSection.settings.title,
                    icon: AppSection.settings.iconSystemName,
                    collapsed: model.sidebarCollapsed,
                    isSelected: model.selectedSection == .settings,
                    theme: model.appTheme,
                    fontChoice: model.fontChoice,
                    circularWhenCollapsed: true
                ) {
                    model.selectedSection = .settings
                }

                SidebarProfileButton(
                    title: AppSection.profile.title,
                    collapsed: model.sidebarCollapsed,
                    isSelected: model.selectedSection == .profile,
                    theme: model.appTheme,
                    fontChoice: model.fontChoice,
                    avatarURL: model.authSession?.avatarURL
                ) {
                    model.selectedSection = .profile
                }
            }
            .frame(maxWidth: .infinity, alignment: model.sidebarCollapsed ? .center : .leading)
            .padding(.bottom, model.sidebarCollapsed ? 0 : 10)
        }
        .padding(.horizontal, model.sidebarCollapsed ? 10 : 12)
        .padding(.bottom, 10)
        .frame(width: model.sidebarCollapsed ? 92.0 : 248.0)
        .frame(maxHeight: .infinity)
        .background {
            Rectangle()
                .fill(.regularMaterial)
                .overlay {
                    Rectangle()
                        .fill(
                            LinearGradient(
                                colors: [
                                    Color.white.opacity(model.darkMode ? 0.08 : 0.38),
                                    accentTint(model.appTheme).opacity(model.darkMode ? 0.08 : 0.04),
                                    Color.clear
                                ],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            )
                        )
                }
        }
        .overlay(alignment: .trailing) {
            Rectangle()
                .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.3))
                .frame(width: 1)
        }
        .ignoresSafeArea(edges: .top)
    }
}

struct SidebarButton: View {
    let title: String
    let icon: String
    let collapsed: Bool
    let isSelected: Bool
    let theme: AppTheme
    let fontChoice: AppFontChoice
    let circularWhenCollapsed: Bool
    let action: () -> Void

    private var darkMode: Bool { theme.isDark }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: icon)
                    .font(.system(size: 14, weight: .bold))
                    .frame(width: 18)
                if !collapsed {
                    Text(title)
                        .font(appFont(fontChoice, .subheadline, weight: .semibold))
                    Spacer(minLength: 0)
                }
            }
            .foregroundStyle(isSelected ? Color.white : Color.primary)
            .padding(.horizontal, collapsed && circularWhenCollapsed ? 0 : 12)
            .padding(.vertical, collapsed && circularWhenCollapsed ? 0 : 9)
            .frame(
                maxWidth: collapsed && circularWhenCollapsed ? 48 : .infinity,
                minHeight: collapsed && circularWhenCollapsed ? 48 : nil,
                alignment: collapsed ? .center : .leading
            )
            .background {
                Group {
                    if collapsed && circularWhenCollapsed {
                        Circle()
                            .fill(
                                isSelected
                                    ? accentTint(theme).opacity(darkMode ? 0.9 : 0.96)
                                    : Color.white.opacity(darkMode ? 0.02 : 0.18)
                            )
                    } else {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .fill(
                                isSelected
                                    ? accentTint(theme).opacity(darkMode ? 0.9 : 0.96)
                                    : Color.white.opacity(darkMode ? 0.02 : 0.18)
                            )
                    }
                }
            }
            .overlay {
                Group {
                    if collapsed && circularWhenCollapsed {
                        Circle()
                            .stroke(
                                isSelected
                                    ? Color.white.opacity(0.16)
                                    : Color.white.opacity(darkMode ? 0.06 : 0.32),
                                lineWidth: 1
                            )
                    } else {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .stroke(
                                isSelected
                                    ? Color.white.opacity(0.16)
                                    : Color.white.opacity(darkMode ? 0.06 : 0.32),
                                lineWidth: 1
                            )
                    }
                }
            }
        }
        .buttonStyle(.plain)
    }
}

struct SidebarProfileButton: View {
    let title: String
    let collapsed: Bool
    let isSelected: Bool
    let theme: AppTheme
    let fontChoice: AppFontChoice
    let avatarURL: URL?
    let action: () -> Void

    private var darkMode: Bool { theme.isDark }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                sidebarAvatar

                if !collapsed {
                    Text(title)
                        .font(appFont(fontChoice, .subheadline, weight: .semibold))
                    Spacer(minLength: 0)
                }
            }
            .foregroundStyle(isSelected ? Color.white : Color.primary)
            .padding(.horizontal, collapsed ? 0 : 12)
            .padding(.vertical, collapsed ? 0 : 9)
            .frame(
                maxWidth: collapsed ? 48 : .infinity,
                minHeight: collapsed ? 48 : nil,
                alignment: collapsed ? .center : .leading
            )
            .background {
                Group {
                    if collapsed {
                        if isSelected {
                            Circle()
                                .fill(accentTint(theme).opacity(darkMode ? 0.9 : 0.96))
                        }
                    } else {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .fill(
                                isSelected
                                    ? accentTint(theme).opacity(darkMode ? 0.9 : 0.96)
                                    : Color.white.opacity(darkMode ? 0.02 : 0.18)
                            )
                    }
                }
            }
            .overlay {
                Group {
                    if !collapsed {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .stroke(Color.white.opacity(darkMode ? 0.06 : 0.26), lineWidth: 1)
                    }
                }
            }
        }
        .buttonStyle(.plain)
    }

    private var sidebarAvatar: some View {
        Group {
            if let avatarURL {
                AsyncImage(url: avatarURL) { phase in
                    switch phase {
                    case let .success(image):
                        image
                            .resizable()
                            .scaledToFill()
                    case .failure, .empty:
                        fallbackAvatar
                    @unknown default:
                        fallbackAvatar
                    }
                }
            } else {
                fallbackAvatar
            }
        }
        .frame(width: collapsed ? 48 : 22, height: collapsed ? 48 : 22)
        .clipShape(Circle())
    }

    private var fallbackAvatar: some View {
        Image(systemName: "person.crop.circle.fill")
            .resizable()
            .scaledToFit()
            .foregroundStyle(isSelected ? Color.white : accentTint(theme))
    }
}

struct AIChatPanelView: View {
    @ObservedObject var model: AppModel
    @ObservedObject var chatModel: AIChatController

    var body: some View {
        expandedPanel
        .frame(width: 360)
        .frame(maxHeight: .infinity)
        .background {
            Rectangle()
                .fill(.regularMaterial)
                .overlay {
                    Rectangle()
                        .fill(
                            LinearGradient(
                                colors: [
                                    Color.white.opacity(model.darkMode ? 0.08 : 0.38),
                                    accentTint(model.appTheme).opacity(model.darkMode ? 0.08 : 0.04),
                                    Color.clear
                                ],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            )
                        )
                }
        }
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.26))
                .frame(width: 1)
        }
        .ignoresSafeArea(edges: .top)
    }

    private var expandedPanel: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("AI Chat")
                        .font(appFont(model.fontChoice, .title3, weight: .bold, scale: model.fontScale))
                }

                Spacer()

                Button {
                    withAnimation(.spring(duration: 0.35, bounce: 0.22)) {
                        chatModel.togglePanel()
                    }
                } label: {
                    Image(systemName: "sidebar.trailing")
                }
                .buttonStyle(.plain)
                .help("Collapse chat")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(.secondary)
            }
            .padding(.top, 12)

            LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 14) {
                HStack(spacing: 10) {
                    Text("Model")
                        .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))

                    Spacer(minLength: 0)

                    Picker("AI Model", selection: $chatModel.selectedModelID) {
                        ForEach(chatModel.availableModels) { model in
                            Text(model.pickerLabel).tag(model.id)
                        }
                    }
                    .pickerStyle(.menu)
                    .onChange(of: chatModel.selectedModelID) { newValue in
                        chatModel.setSelectedModel(newValue)
                    }
                }
            }

            LiquidCard(theme: model.appTheme, cornerRadius: 22, padding: 0) {
                VStack(spacing: 0) {
                    ScrollViewReader { proxy in
                        ScrollView {
                            VStack(alignment: .leading, spacing: 10) {
                                ForEach(chatModel.messages) { message in
                                    AIChatBubble(
                                        model: model,
                                        message: message
                                    )
                                    .id(message.id)
                                }

                                if chatModel.isSending {
                                    HStack(spacing: 8) {
                                        ProgressView()
                                            .controlSize(.small)
                                        Text("Waiting for \(chatModel.selectedModel.title)...")
                                            .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                                            .foregroundStyle(.secondary)
                                    }
                                    .padding(.horizontal, 12)
                                    .padding(.vertical, 10)
                                }
                            }
                            .padding(12)
                        }
                        .onChange(of: chatModel.messages.count) { _ in
                            if let lastMessageID = chatModel.messages.last?.id {
                                withAnimation(.easeOut(duration: 0.2)) {
                                    proxy.scrollTo(lastMessageID, anchor: .bottom)
                                }
                            }
                        }
                    }

                    Rectangle()
                        .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.18))
                        .frame(height: 1)

                    VStack(alignment: .leading, spacing: 10) {
                        Text(chatModel.statusMessage)
                            .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)

                        ChatComposerTextView(
                            text: $chatModel.draftMessage,
                            font: appNSFont(model.fontChoice, size: NSFont.systemFontSize, scale: model.fontScale),
                            onSubmit: {
                                chatModel.sendDraftMessage(packetContext: model.selectedPacketContextForAI)
                            }
                        )
                            .frame(minHeight: 44, maxHeight: 60)
                            .padding(8)
                            .background(
                                RoundedRectangle(cornerRadius: 16, style: .continuous)
                                    .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.42))
                            )

                        HStack {
                            Spacer()

                            Button {
                                chatModel.sendDraftMessage(packetContext: model.selectedPacketContextForAI)
                            } label: {
                                HStack(spacing: 8) {
                                    Image(systemName: "arrow.up.circle.fill")
                                    Text(chatModel.isSending ? "Sending" : "Send")
                                }
                                .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                                .background(
                                    Capsule(style: .continuous)
                                        .fill(accentTint(model.appTheme))
                                )
                                .foregroundStyle(Color.white)
                            }
                            .buttonStyle(.plain)
                            .disabled(chatModel.isSending)
                        }
                    }
                    .padding(12)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .padding(.horizontal, 12)
        .padding(.top, 0)
        .padding(.bottom, 12)
        .ignoresSafeArea(edges: .top)
    }
}

struct AIChatCollapsedLauncher: View {
    @ObservedObject var model: AppModel
    @ObservedObject var chatModel: AIChatController

    var body: some View {
        Button {
            withAnimation(.spring(duration: 0.35, bounce: 0.22)) {
                chatModel.togglePanel()
            }
        } label: {
            VStack(spacing: 10) {
                Image(systemName: "sparkles")
                    .font(.system(size: 18, weight: .bold))

                Text("AI")
                    .font(appFont(model.fontChoice, .caption, weight: .semibold, scale: model.fontScale))

                Divider()
                    .overlay(Color.white.opacity(0.2))

                Image(systemName: chatModel.selectedModel.provider.symbolName)
                    .font(.system(size: 13, weight: .semibold))

                Text(chatModel.selectedModel.provider.title)
                    .font(appFont(model.fontChoice, .caption2, weight: .semibold, scale: model.fontScale))
                    .multilineTextAlignment(.center)
                    .lineLimit(2)
            }
            .foregroundStyle(Color.white)
            .padding(.horizontal, 10)
            .padding(.vertical, 14)
            .frame(width: 58)
            .background(
                RoundedRectangle(cornerRadius: 20, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [
                                accentTint(model.appTheme),
                                accentTint(model.appTheme).opacity(0.72)
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .overlay {
                RoundedRectangle(cornerRadius: 20, style: .continuous)
                    .stroke(Color.white.opacity(0.18), lineWidth: 1)
            }
            .shadow(color: Color.black.opacity(model.darkMode ? 0.28 : 0.12), radius: 18, x: 0, y: 10)
        }
        .buttonStyle(.plain)
    }
}

private struct AIChatSettingsCard: View {
    @ObservedObject var model: AppModel
    @ObservedObject var chatModel: AIChatController

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 14) {
            VStack(alignment: .leading, spacing: 14) {
                Text("Provider Keys")
                    .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))

                Text("API keys are stored in macOS Keychain. Saved values are never shown again in plain text here.")
                    .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                    .foregroundStyle(.secondary)

                AIProviderKeyRow(
                    model: model,
                    provider: .openAI,
                    keyDraft: $chatModel.openAIApiKeyDraft,
                    isConfigured: chatModel.apiKeyConfigured(for: .openAI),
                    saveAction: { chatModel.saveAPIKeyDraft(for: .openAI) },
                    removeAction: { chatModel.removeAPIKey(for: .openAI) }
                )

                AIProviderKeyRow(
                    model: model,
                    provider: .anthropic,
                    keyDraft: $chatModel.anthropicApiKeyDraft,
                    isConfigured: chatModel.apiKeyConfigured(for: .anthropic),
                    saveAction: { chatModel.saveAPIKeyDraft(for: .anthropic) },
                    removeAction: { chatModel.removeAPIKey(for: .anthropic) }
                )

                AIProviderKeyRow(
                    model: model,
                    provider: .google,
                    keyDraft: $chatModel.googleApiKeyDraft,
                    isConfigured: chatModel.apiKeyConfigured(for: .google),
                    saveAction: { chatModel.saveAPIKeyDraft(for: .google) },
                    removeAction: { chatModel.removeAPIKey(for: .google) }
                )

                Text("Local Subscription Providers")
                    .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                    .padding(.top, 4)

                Text("These routes use the CLI already installed and signed in on this Mac. No API key is stored here.")
                    .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                    .foregroundStyle(.secondary)

                AILocalRuntimeRow(
                    model: model,
                    provider: .codex,
                    isAvailable: chatModel.localRuntimeConfigured(for: .codex),
                    refreshAction: { chatModel.refreshLocalRuntimeAvailability() },
                    helperText: "Uses the local `codex` session. Sign in once in Terminal with your ChatGPT-backed Codex account."
                )

                AILocalRuntimeRow(
                    model: model,
                    provider: .claudeCode,
                    isAvailable: chatModel.localRuntimeConfigured(for: .claudeCode),
                    refreshAction: { chatModel.refreshLocalRuntimeAvailability() },
                    helperText: "Uses the local `claude` session. Sign in once in Terminal with your Claude Pro or Max account."
                )
            }
        }
    }
}

private struct AIProviderKeyRow: View {
    @ObservedObject var model: AppModel
    let provider: AIChatProvider
    @Binding var keyDraft: String
    let isConfigured: Bool
    let saveAction: () -> Void
    let removeAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label(provider.title, systemImage: provider.symbolName)
                    .font(appFont(model.fontChoice, .subheadline, weight: .semibold, scale: model.fontScale))

                Spacer()

                Text(isConfigured ? "Saved" : "Not Saved")
                    .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                    .foregroundStyle(isConfigured ? .green : .secondary)
            }

            HStack(spacing: 8) {
                SecureField("\(provider.title) API key", text: $keyDraft)
                    .textFieldStyle(.roundedBorder)

                Button("Save") {
                    saveAction()
                }
                .buttonStyle(.borderedProminent)
                .tint(accentTint(model.appTheme))

                Button("Remove") {
                    removeAction()
                }
                .buttonStyle(.bordered)
                .disabled(!isConfigured)
            }
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.34))
        )
    }
}

private struct AILocalRuntimeRow: View {
    @ObservedObject var model: AppModel
    let provider: AIChatProvider
    let isAvailable: Bool
    let refreshAction: () -> Void
    let helperText: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label(provider.title, systemImage: provider.symbolName)
                    .font(appFont(model.fontChoice, .subheadline, weight: .semibold, scale: model.fontScale))

                Spacer()

                Text(isAvailable ? "Installed" : "Missing")
                    .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                    .foregroundStyle(isAvailable ? .green : .orange)
            }

            Text(helperText)
                .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            Button("Refresh Availability") {
                refreshAction()
            }
            .buttonStyle(.bordered)
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.34))
        )
    }
}

private struct AIChatBubble: View {
    @ObservedObject var model: AppModel
    let message: AIChatMessage

    private var isUser: Bool {
        message.role == .user
    }

    var body: some View {
        HStack {
            if isUser {
                Spacer(minLength: 26)
            }

            VStack(alignment: .leading, spacing: 0) {
                Text(message.content)
                    .font(appFont(model.fontChoice, .body, scale: model.fontScale))
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .fill(
                        isUser
                            ? accentTint(model.appTheme).opacity(model.darkMode ? 0.82 : 0.9)
                            : Color.white.opacity(model.darkMode ? 0.08 : 0.5)
                    )
            )
            .foregroundStyle(isUser ? Color.white : Color.primary)

            if !isUser {
                Spacer(minLength: 26)
            }
        }
    }
}

struct DetailView: View {
    @ObservedObject var model: AppModel
    @ObservedObject var chatModel: AIChatController
    let saveCaptureAction: () -> Void
    let openCaptureAction: () -> Void

    var body: some View {
        VStack(spacing: 12) {
            HStack(alignment: .center, spacing: 10) {
                HStack(spacing: 10) {
                    Text(model.selectedSection.title)
                        .font(appFont(model.fontChoice, .title2, weight: .bold, scale: model.fontScale))

                    if model.isBusy {
                        ProgressView()
                            .controlSize(.small)
                    }
                }
                .padding(.leading, model.sidebarCollapsed ? 42 : 0)

                Spacer()

                HStack(spacing: 8) {
                    Text(model.statusMessage)
                        .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)

                    Button {
                        copyToPasteboard(model.statusMessage)
                    } label: {
                        Image(systemName: "doc.on.doc")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                    .help("Copy status message")

                    if chatModel.panelCollapsed {
                        AIChatHeaderLauncher(
                            model: model,
                            chatModel: chatModel
                        )
                    }
                }
            }

            LiquidCard(theme: model.appTheme, cornerRadius: 28, padding: 16) {
                Group {
                    switch model.selectedSection {
                    case .packets:
                        PacketsSectionView(
                            model: model,
                            saveCaptureAction: saveCaptureAction,
                            openCaptureAction: openCaptureAction
                        )
                    case .stats:
                        StatsSectionView(model: model)
                    case .conversations:
                        ConversationsSectionView(model: model)
                    case .streams:
                        StreamsSectionView(model: model)
                    case .transactions:
                        TransactionsSectionView(model: model)
                    case .profile:
                        ProfileSectionView(model: model)
                    case .settings:
                        SettingsSectionView(model: model, chatModel: chatModel)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .padding(.top, -40)
        .padding(.horizontal, 14)
        .padding(.bottom, 0)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .sheet(item: $model.packetInspectorState) { inspector in
            PacketInspectorWindow(inspector: inspector, darkMode: model.darkMode)
        }
    }
}

private struct AIChatHeaderLauncher: View {
    @ObservedObject var model: AppModel
    @ObservedObject var chatModel: AIChatController

    var body: some View {
        Button {
            withAnimation(.spring(duration: 0.35, bounce: 0.22)) {
                chatModel.togglePanel()
            }
        } label: {
            Image(systemName: "sparkles")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.secondary)
                .frame(width: 26, height: 26)
                .background(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.34))
                )
                .overlay {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(Color.white.opacity(model.darkMode ? 0.08 : 0.24), lineWidth: 1)
                }
        }
        .buttonStyle(.plain)
        .help("Show AI chat")
    }
}

struct MetricCard: View {
    let title: String
    let value: String
    let darkMode: Bool
    let fontChoice: AppFontChoice

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(appFont(fontChoice, .caption, weight: .medium))
                .foregroundStyle(.secondary)
            Text(value)
                .font(appFont(fontChoice, .title3, weight: .bold))
                .lineLimit(1)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(darkMode ? 0.08 : 0.44))
                .overlay {
                    RoundedRectangle(cornerRadius: 16, style: .continuous)
                        .stroke(Color.white.opacity(darkMode ? 0.16 : 0.58), lineWidth: 1)
                }
        }
    }
}

struct PacketsSectionView: View {
    @ObservedObject var model: AppModel
    let saveCaptureAction: () -> Void
    let openCaptureAction: () -> Void
    @State private var filterRefreshTask: Task<Void, Never>?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .top, spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text("Filter")
                            .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                    }

                    HStack(spacing: 10) {
                        NativeTextField(
                            placeholder: "protocol & port",
                            text: $model.filterExpression,
                            font: appNSFont(model.fontChoice == .monospaced ? .monospaced : .rounded, size: NSFont.systemFontSize, scale: model.fontScale)
                        )
                        .frame(height: 24)
                    }
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .topLeading)
                .background(
                    RoundedRectangle(cornerRadius: 20, style: .continuous)
                        .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.5))
                )

                VStack(alignment: .leading, spacing: 12) {
                    HStack(alignment: .center) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Live Capture")
                                .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                            Text(model.captureBackendMessage)
                                .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                                .foregroundStyle(.secondary)
                        }

                        Spacer()

                        Text(model.isSniffing ? "Running (\(model.captureBackendName))" : "Idle (\(model.captureBackendName))")
                            .font(appFont(model.fontChoice, .caption, weight: .bold, scale: model.fontScale))
                    }

                    Picker("Capture Interface", selection: $model.selectedCaptureInterface) {
                        ForEach(model.availableCaptureInterfaces, id: \.self) { interface in
                            Text(interface).tag(interface)
                        }
                    }
                    .pickerStyle(.menu)

                    HStack(spacing: 10) {
                        Button(model.isSniffing ? "Stop Sniffing" : "Start Sniffing") {
                            model.toggleSniffing()
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(model.isSniffing ? .red : accentTint(model.appTheme))
                        .disabled(model.captureBackendName == "Unavailable")

                        Button("Save Capture") {
                            saveCaptureAction()
                        }

                        Button("Open Capture") {
                            openCaptureAction()
                        }
                    }
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .topLeading)
                .background(
                    RoundedRectangle(cornerRadius: 20, style: .continuous)
                        .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.5))
                )
            }

            HStack(alignment: .top) {
                PacketCounterCard(model: model)
                    .padding(.top, -18)

                Spacer(minLength: 0)
            }

            HStack(spacing: 12) {
                LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 12) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Packets")
                            .font(.system(.headline, design: .rounded).weight(.bold))

                        List(selection: $model.selectedPacketIndex) {
                            ForEach(model.packets) { packet in
                                VStack(alignment: .leading, spacing: 4) {
                                    HStack {
                                        Text("#\(packet.index)")
                                            .font(.system(.caption, design: .monospaced))
                                            .foregroundStyle(.secondary)
                                        Text(packet.protocolName.uppercased())
                                            .font(.system(.caption2, design: .rounded).weight(.bold))
                                            .padding(.horizontal, 7)
                                            .padding(.vertical, 2)
                                            .background(Capsule().fill(Color.white.opacity(model.darkMode ? 0.1 : 0.4)))
                                        Spacer()
                                        Text(packet.timestamp)
                                            .font(.system(.caption2, design: .monospaced))
                                            .foregroundStyle(.secondary)
                                    }

                                    Text("\(packet.source) → \(packet.destination)")
                                        .font(.system(.caption, design: .monospaced))
                                        .lineLimit(1)

                                    Text(packet.info)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }
                                .padding(.vertical, 4)
                                .tag(packet.index)
                                .listRowBackground(Color.clear)
                                .listRowSeparator(.hidden)
                                .contextMenu {
                                    Button("Show Packet Details") {
                                        model.presentPacketInspector(index: packet.index)
                                    }
                                }
                            }
                        }
                        .listStyle(.plain)
                        .scrollContentBackground(.hidden)
                        .onChange(of: model.selectedPacketIndex) { value in
                            guard let index = value else { return }
                            model.loadPacketDetails(index: index)
                        }
                    }
                }
                .frame(minWidth: 540)

                LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 12) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Packet JSON")
                            .font(.system(.headline, design: .rounded).weight(.bold))

                        ScrollView {
                            Text(model.selectedPacketJSON)
                                .font(.system(.caption, design: .monospaced))
                                .frame(maxWidth: .infinity, alignment: .topLeading)
                                .textSelection(.enabled)
                                .padding(12)
                        }
                        .background(Color.white.opacity(model.darkMode ? 0.06 : 0.38))
                        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .onChange(of: model.filterExpression) { _ in
            schedulePacketRefresh()
        }
    }

    private func schedulePacketRefresh() {
        filterRefreshTask?.cancel()
        guard !model.capturePath.isEmpty else { return }

        filterRefreshTask = Task {
            try? await Task.sleep(for: .milliseconds(350))
            guard !Task.isCancelled else { return }
            await MainActor.run {
                model.refreshAll()
            }
        }
    }
}

struct PacketCounterCard: View {
    @ObservedObject var model: AppModel

    var body: some View {
        HStack(spacing: 8) {
            Text("Packets")
                .font(appFont(model.fontChoice, .caption, weight: .semibold, scale: model.fontScale))
                .foregroundStyle(.secondary)

            Text("\(model.totalPackets)")
                .font(appFont(model.fontChoice, .title3, weight: .bold, scale: model.fontScale))
        }
    }
}

struct PacketInspectorWindow: View {
    let inspector: PacketInspectorState
    let darkMode: Bool
    @State private var selectedFieldID: UUID?

    private var selectedField: PacketFieldNode? {
        guard let selectedFieldID else { return inspector.flatFields.first?.node }
        return inspector.flatFields.first(where: { $0.id == selectedFieldID })?.node
    }

    private var selectedRange: ByteRangeMetadata? {
        selectedField?.byteRange
    }

    private var selectedBytes: [UInt8] {
        guard let range = selectedRange else { return inspector.rawBytes }
        let lowerBound = max(0, min(range.start, inspector.rawBytes.count))
        let upperBound = max(lowerBound, min(range.end, inspector.rawBytes.count))
        return Array(inspector.rawBytes[lowerBound..<upperBound])
    }

    var body: some View {
        HSplitView {
            VStack(alignment: .leading, spacing: 12) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Packet \(inspector.packetNumber)")
                        .font(.system(.title2, design: .rounded).weight(.bold))

                    PacketInspectorSummaryRow(title: "Timestamp", value: inspector.timestamp)
                    PacketInspectorSummaryRow(title: "Size", value: "\(inspector.capturedLength) captured / \(inspector.originalLength) original")
                    PacketInspectorSummaryRow(title: "Link", value: inspector.linkSummary)
                    PacketInspectorSummaryRow(title: "Network", value: inspector.networkSummary)
                    PacketInspectorSummaryRow(title: "Transport", value: inspector.transportSummary)
                    PacketInspectorSummaryRow(title: "Application", value: inspector.applicationSummary)
                }

                Text("Decoded Fields")
                    .font(.system(.headline, design: .rounded).weight(.semibold))

                List(selection: $selectedFieldID) {
                    ForEach(inspector.flatFields) { item in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.node.name)
                                .font(.system(.subheadline, design: .rounded).weight(.semibold))
                            if !item.node.value.isEmpty {
                                Text(item.node.value)
                                    .font(.system(.caption, design: .monospaced))
                                    .foregroundStyle(.secondary)
                                    .lineLimit(2)
                            }
                        }
                        .padding(.leading, CGFloat(item.depth * 14))
                        .padding(.vertical, 3)
                        .tag(item.id)
                    }
                }
                .listStyle(.inset)
            }
            .frame(minWidth: 360)
            .padding(18)

            VStack(alignment: .leading, spacing: 12) {
                Text("Bytes / Hex")
                    .font(.system(.headline, design: .rounded).weight(.semibold))

                if let selectedField {
                    PacketInspectorSummaryRow(title: "Field", value: selectedField.name)
                    PacketInspectorSummaryRow(title: "Value", value: selectedField.value.isEmpty ? "—" : selectedField.value)
                    PacketInspectorSummaryRow(
                        title: "Range",
                        value: selectedRange.map { "\($0.start)...\($0.end) (\($0.count) bytes)" } ?? "Whole packet"
                    )
                }

                ScrollView {
                    Text(hexDump(for: selectedBytes))
                        .font(.system(.caption, design: .monospaced))
                        .frame(maxWidth: .infinity, alignment: .topLeading)
                        .textSelection(.enabled)
                        .padding(12)
                }
                .background(
                    RoundedRectangle(cornerRadius: 16, style: .continuous)
                        .fill(Color.white.opacity(darkMode ? 0.06 : 0.42))
                )
            }
            .frame(minWidth: 420)
            .padding(18)
        }
        .frame(minWidth: 860, minHeight: 560)
        .onAppear {
            selectedFieldID = inspector.flatFields.first?.id
        }
    }

    private func hexDump(for bytes: [UInt8]) -> String {
        guard !bytes.isEmpty else { return "No bytes available." }
        return stride(from: 0, to: bytes.count, by: 16)
            .map { offset in
                let chunk = bytes[offset..<min(offset + 16, bytes.count)]
                let hex = chunk.map { String(format: "%02X", $0) }.joined(separator: " ")
                return String(format: "%04X  %@", offset, hex)
            }
            .joined(separator: "\n")
    }
}

struct PacketInspectorSummaryRow: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            Text(value)
                .font(.system(.subheadline, design: .monospaced))
                .textSelection(.enabled)
        }
    }
}

struct StatsSectionView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 12) {
            VStack(alignment: .leading, spacing: 8) {
                Text("Protocol Distribution")
                    .font(.system(.headline, design: .rounded).weight(.bold))

                List(model.statsRows) { row in
                    HStack {
                        Text(row.bucket.uppercased())
                            .font(.system(.caption2, design: .rounded).weight(.bold))
                            .foregroundStyle(.secondary)
                            .frame(width: 76, alignment: .leading)
                        Text(row.name)
                            .font(.system(.body, design: .monospaced))
                        Spacer()
                        Text("\(row.count)")
                            .font(.system(.body, design: .monospaced).weight(.semibold))
                    }
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                }
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
            }
        }
    }
}

struct ConversationsSectionView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 12) {
            VStack(alignment: .leading, spacing: 8) {
                Text("Conversations")
                    .font(.system(.headline, design: .rounded).weight(.bold))

                List(model.conversations) { row in
                    HStack(spacing: 10) {
                        Text(row.protocolName.uppercased())
                            .font(.system(.caption2, design: .rounded).weight(.bold))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Capsule().fill(Color.white.opacity(model.darkMode ? 0.1 : 0.35)))
                        Text("\(row.endpointA) ↔ \(row.endpointB)")
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1)
                        Spacer()
                        Text("\(row.packets)")
                            .font(.system(.caption, design: .monospaced).weight(.semibold))
                    }
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                }
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
            }
        }
    }
}

struct StreamsSectionView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 12) {
            VStack(alignment: .leading, spacing: 8) {
                Text("Streams")
                    .font(.system(.headline, design: .rounded).weight(.bold))

                List(model.streams) { row in
                    VStack(alignment: .leading, spacing: 3) {
                        HStack {
                            Text(row.protocolName.uppercased())
                                .font(.system(.caption2, design: .rounded).weight(.bold))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Capsule().fill(Color.white.opacity(model.darkMode ? 0.1 : 0.35)))
                            Text(row.state)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Spacer()
                            Text("packets: \(row.packets)")
                                .font(.system(.caption, design: .monospaced))
                        }
                        Text("\(row.client) → \(row.server)")
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1)
                    }
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                }
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
            }
        }
    }
}

struct TransactionsSectionView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 12) {
            VStack(alignment: .leading, spacing: 8) {
                Text("Transactions")
                    .font(.system(.headline, design: .rounded).weight(.bold))

                List(model.transactions) { row in
                    VStack(alignment: .leading, spacing: 3) {
                        HStack {
                            Text(row.protocolName.uppercased())
                                .font(.system(.caption2, design: .rounded).weight(.bold))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Capsule().fill(Color.white.opacity(model.darkMode ? 0.1 : 0.35)))
                            Text(row.state)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Spacer()
                        }
                        Text("REQ: \(row.requestSummary)")
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1)
                        Text("RES: \(row.responseSummary)")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                }
                .listStyle(.plain)
                .scrollContentBackground(.hidden)
            }
        }
    }
}

struct ProfileSectionView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 18) {
            VStack(alignment: .leading, spacing: 18) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Profile")
                        .font(appFont(model.fontChoice, .title3, weight: .bold, scale: model.fontScale))
                    Text("Account identity and local preferences")
                        .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                        .foregroundStyle(.secondary)
                }

                if let session = model.authSession {
                    signedInState(session: session)
                } else {
                    signedOutState
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    private var signedOutState: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(model.cloudProfilesAvailable
                ? "Sign in to identify yourself in IceSniff. Preferences are stored locally on this Mac only."
                : "Sign-in is unavailable in this build. Preferences are stored locally on this Mac only.")
                .font(appFont(model.fontChoice, .subheadline, scale: model.fontScale))
                .foregroundStyle(.secondary)

            if model.cloudProfilesAvailable {
                HStack(spacing: 12) {
                    ForEach(AuthProvider.allCases) { provider in
                        Button {
                            model.signIn(with: provider)
                        } label: {
                            HStack(spacing: 8) {
                                Image(systemName: provider.symbolName)
                                Text("Continue with \(provider.title)")
                            }
                            .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                            .padding(.horizontal, 14)
                            .padding(.vertical, 10)
                            .background(
                                Capsule(style: .continuous)
                                    .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.44))
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }

            profileStatusCard
        }
    }

    private func signedInState(session: AuthSession) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .top) {
                profileAvatar(for: session)

                VStack(alignment: .leading, spacing: 4) {
                    Text(session.displayName ?? "Signed In")
                        .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                    if let email = session.email {
                        Text(email)
                            .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                            .foregroundStyle(.secondary)
                    }
                    Text("Provider: \(session.provider.title)")
                        .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Button("Sign Out") {
                    model.signOutProfile()
                }
                .buttonStyle(.plain)
                .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(
                    Capsule(style: .continuous)
                        .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.4))
                )
            }

            HStack(spacing: 12) {
                statCard(title: "Theme", value: model.appTheme.title)
                statCard(title: "Font", value: model.fontChoice.title)
                statCard(title: "Size", value: String(describing: model.fontSizeStep.rawValue))
            }

            profileStatusCard
        }
    }

    @ViewBuilder
    private func profileAvatar(for session: AuthSession) -> some View {
        Group {
            if let avatarURL = session.avatarURL {
                AsyncImage(url: avatarURL) { phase in
                    switch phase {
                    case let .success(image):
                        image
                            .resizable()
                            .scaledToFill()
                    case .failure, .empty:
                        Image(systemName: "person.crop.circle.fill")
                            .resizable()
                            .scaledToFit()
                            .padding(8)
                            .foregroundStyle(accentTint(model.appTheme))
                    @unknown default:
                        Image(systemName: "person.crop.circle.fill")
                            .resizable()
                            .scaledToFit()
                            .padding(8)
                            .foregroundStyle(accentTint(model.appTheme))
                    }
                }
            } else {
                Image(systemName: "person.crop.circle.fill")
                    .resizable()
                    .scaledToFit()
                    .padding(8)
                    .foregroundStyle(accentTint(model.appTheme))
            }
        }
        .frame(width: 56, height: 56)
        .background(
            Circle()
                .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.4))
        )
        .clipShape(Circle())
    }

    private var profileStatusCard: some View {
        RoundedRectangle(cornerRadius: 16, style: .continuous)
            .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.4))
            .overlay(alignment: .leading) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 8) {
                        Text("Profile Status")
                            .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))

                        Spacer(minLength: 0)

                        Button {
                            copyToPasteboard(model.profileStatusMessage)
                        } label: {
                            Image(systemName: "doc.on.doc")
                                .font(.system(size: 11, weight: .semibold))
                                .foregroundStyle(.secondary)
                        }
                        .buttonStyle(.plain)
                        .help("Copy profile status")
                    }

                    Text(model.profileStatusMessage)
                        .font(appFont(model.fontChoice, .caption, scale: model.fontScale))
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(14)
            }
            .frame(maxWidth: .infinity, minHeight: 92)
    }

    private func statCard(title: String, value: String) -> some View {
        RoundedRectangle(cornerRadius: 16, style: .continuous)
            .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.4))
            .overlay(alignment: .leading) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                        .font(appFont(model.fontChoice, .caption, weight: .medium, scale: model.fontScale))
                        .foregroundStyle(.secondary)
                    Text(value)
                        .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))
                }
                .padding(14)
            }
            .frame(maxWidth: .infinity, minHeight: 72)
    }
}

struct SettingsSectionView: View {
    @ObservedObject var model: AppModel
    @ObservedObject var chatModel: AIChatController

    var body: some View {
        LiquidCard(theme: model.appTheme, cornerRadius: 20, padding: 18) {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text("Settings")
                        .font(appFont(model.fontChoice, .title3, weight: .bold))

                    VStack(alignment: .leading, spacing: 18) {
                        VStack(alignment: .leading, spacing: 12) {
                            Text("Theme")
                                .font(appFont(model.fontChoice, .headline, weight: .semibold))

                            HStack(spacing: 10) {
                                ForEach(AppTheme.allCases) { theme in
                                    VStack(spacing: 8) {
                                        RoundedRectangle(cornerRadius: 14, style: .continuous)
                                            .fill(
                                                LinearGradient(
                                                    colors: palette(for: theme).backgroundGradient,
                                                    startPoint: .topLeading,
                                                    endPoint: .bottomTrailing
                                                )
                                            )
                                            .overlay(alignment: .bottomTrailing) {
                                                Circle()
                                                    .fill(accentTint(theme))
                                                    .frame(width: 14, height: 14)
                                                    .padding(8)
                                            }
                                            .overlay {
                                                RoundedRectangle(cornerRadius: 14, style: .continuous)
                                                    .stroke(theme == model.appTheme ? accentTint(model.appTheme) : Color.white.opacity(model.darkMode ? 0.12 : 0.3), lineWidth: theme == model.appTheme ? 2 : 1)
                                            }
                                            .frame(width: 104, height: 72)
                                            .onTapGesture {
                                                model.setTheme(theme)
                                            }

                                        Text(theme.title)
                                            .font(appFont(model.fontChoice, .caption, weight: .medium))
                                            .foregroundStyle(.secondary)
                                    }
                                }
                            }
                        }
                        VStack(alignment: .leading, spacing: 12) {
                            HStack(spacing: 8) {
                                Text("Font")
                                    .font(appFont(model.fontChoice, .headline, weight: .semibold, scale: model.fontScale))

                                Spacer()

                                Button {
                                    model.decreaseFontSize()
                                } label: {
                                    Image(systemName: "textformat.size.smaller")
                                        .font(.system(size: 14, weight: .bold))
                                        .frame(width: 34, height: 30)
                                        .background(
                                            Capsule(style: .continuous)
                                                .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.42))
                                        )
                                }
                                .buttonStyle(.plain)
                                .opacity(model.fontSizeStep == .extraSmall ? 0.45 : 1)
                                .disabled(model.fontSizeStep == .extraSmall)

                                Button {
                                    model.increaseFontSize()
                                } label: {
                                    Image(systemName: "textformat.size.larger")
                                        .font(.system(size: 14, weight: .bold))
                                        .frame(width: 34, height: 30)
                                        .background(
                                            Capsule(style: .continuous)
                                                .fill(Color.white.opacity(model.darkMode ? 0.08 : 0.42))
                                        )
                                }
                                .buttonStyle(.plain)
                                .opacity(model.fontSizeStep == .extraLarge ? 0.45 : 1)
                                .disabled(model.fontSizeStep == .extraLarge)
                            }

                            VStack(alignment: .leading, spacing: 10) {
                                ForEach(AppFontChoice.allCases) { choice in
                                    Button {
                                        model.setFontChoice(choice)
                                    } label: {
                                        HStack {
                                            VStack(alignment: .leading, spacing: 3) {
                                                Text(choice.title)
                                                    .font(appFont(choice, .headline, weight: .semibold))
                                                Text("The quick brown fox jumps over the lazy packet.")
                                                    .font(appFont(choice, .caption))
                                                    .foregroundStyle(.secondary)
                                                    .lineLimit(1)
                                            }
                                            Spacer()
                                            if choice == model.fontChoice {
                                                Image(systemName: "checkmark.circle.fill")
                                                    .foregroundStyle(accentTint(model.appTheme))
                                            }
                                        }
                                        .padding(12)
                                        .background(
                                            RoundedRectangle(cornerRadius: 16, style: .continuous)
                                                .fill(Color.white.opacity(model.darkMode ? 0.06 : 0.4))
                                        )
                                    }
                                    .buttonStyle(.plain)
                                }
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .topLeading)

                        AIChatSettingsCard(model: model, chatModel: chatModel)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .topLeading)
            }
            .scrollIndicators(.visible)
        }
    }
}
