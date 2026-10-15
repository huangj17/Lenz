import Foundation
import AppKit
import SwiftUI
import Translation
import NaturalLanguage

struct Input: Decodable {
    let texts: [String]
    let sourceLang: String?
    let targetLang: String
}
struct Output: Encodable {
    var translations: [String] = []
    var detectedLanguage: String? = nil
    var error: String? = nil
}
func finish(_ result: Output) {
    if let data = try? JSONEncoder().encode(result) {
        FileHandle.standardOutput.write(data)
    }
    exit(result.error == nil ? 0 : 1)
}
@available(macOS 15.0, *)
struct TranslationView: View {
    let input: Input
    let source: Locale.Language?
    var body: some View {
        VStack(spacing: 16) {
            ProgressView()
            Text("Lenz · Apple Translation")
        }.padding(32)
        .translationTask(source: source, target: Locale.Language(identifier: input.targetLang)) { session in
            do {
                var results: [String] = []
                for text in input.texts {
                    results.append(try await session.translate(text).targetText)
                }
                finish(Output(translations: results, detectedLanguage: source?.minimalIdentifier))
            } catch { finish(Output(error: error.localizedDescription)) }
        }
    }
}
@MainActor
final class CloseDelegate: NSObject, NSWindowDelegate {
    func windowWillClose(_ notification: Notification) { finish(Output(error: "Translation cancelled")) }
}
@main
struct Runner {
    @MainActor
    static func main() async {
        do {
            let input = try JSONDecoder().decode(Input.self, from: FileHandle.standardInput.readDataToEndOfFile())
            guard #available(macOS 15.0, *) else {
                finish(Output(error: "Apple Translation requires macOS 15 or later."))
                return
            }
            let recognizer = NLLanguageRecognizer()
            recognizer.processString(input.texts.joined(separator: "\n"))
            let sourceTag = input.sourceLang ?? recognizer.dominantLanguage?.rawValue
            let source = sourceTag.map { Locale.Language(identifier: $0) }
            let target = Locale.Language(identifier: input.targetLang)
            if let source, source.languageCode != nil, source.languageCode == target.languageCode {
                finish(Output(translations: input.texts, detectedLanguage: sourceTag))
                return
            }
            guard let source else {
                finish(Output(error: TranslationError.unableToIdentifyLanguage.localizedDescription))
                return
            }
            // A missing pack can leave Apple's download consent sheet spinning
            // indefinitely. Check before opening a host or creating a session;
            // language installation belongs in System Settings, outside the request timeout.
            switch await LanguageAvailability().status(from: source, to: target) {
            case .supported:
                finish(Output(error: "apple_languages_required: Download the source and target languages in System Settings > General > Language & Region > Translation Languages, then retry."))
                return
            case .unsupported:
                finish(Output(error: TranslationError.unsupportedLanguagePairing.localizedDescription))
                return
            case .installed:
                break
            @unknown default:
                finish(Output(error: TranslationError.internalError.localizedDescription))
                return
            }
            // macOS 26 offers an installed-language session without a SwiftUI host.
            if #available(macOS 26.0, *) {
                let session = TranslationSession(installedSource: source, target: target)
                var results: [String] = []
                for text in input.texts { results.append(try await session.translate(text).targetText) }
                finish(Output(translations: results, detectedLanguage: sourceTag))
                return
            }
            // macOS 15 needs a SwiftUI host even when the languages are installed.
            let app = NSApplication.shared
            app.setActivationPolicy(.accessory)
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 300, height: 160),
                styleMask: [.titled, .closable], backing: .buffered, defer: false)
            window.title = "Lenz · Apple Translation"
            window.contentView = NSHostingView(rootView: TranslationView(input: input, source: source))
            window.center()
            window.makeKeyAndOrderFront(nil)
            app.activate(ignoringOtherApps: true)
            let delegate = CloseDelegate()
            window.delegate = delegate
            withExtendedLifetime(delegate) { app.run() }
        } catch { finish(Output(error: error.localizedDescription)) }
    }
}
