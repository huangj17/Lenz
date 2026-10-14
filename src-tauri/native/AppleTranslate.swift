import Foundation
import AppKit
import SwiftUI
import Translation
import NaturalLanguage

struct Input: Decodable {
    let texts: [String]
    let sourceLang: String?
    let targetLang: String
    let prepare: Bool?
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
                if input.prepare == true {
                    try await session.prepareTranslation()
                    finish(Output())
                }
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
            if let source, source.languageCode != nil, source.languageCode == Locale.Language(identifier: input.targetLang).languageCode, input.prepare != true {
                finish(Output(translations: input.texts, detectedLanguage: sourceTag))
                return
            }
            // macOS 26 offers an installed-language session without a SwiftUI host.
            if #available(macOS 26.0, *), input.prepare != true, let source {
                let session = TranslationSession(installedSource: source, target: Locale.Language(identifier: input.targetLang))
                if await session.isReady {
                    var results: [String] = []
                    for text in input.texts { results.append(try await session.translate(text).targetText) }
                    finish(Output(translations: results, detectedLanguage: sourceTag))
                    return
                }
            }
            // Earlier systems / missing packs use Apple's normal download consent UI.
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
