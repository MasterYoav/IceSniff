import Foundation
import Testing
@testable import IceSniffMac

struct AppModelTests {
    @Test
    func filterNormalizationSupportsComfortSyntax() {
        #expect(FilterExpressionNormalizer.normalize("HTTP") == "protocol=http")
        #expect(FilterExpressionNormalizer.normalize("udp and 443") == "protocol=udp && port=443")
        #expect(FilterExpressionNormalizer.normalize("tcp & 80") == "protocol=tcp && port=80")
        #expect(FilterExpressionNormalizer.normalize("protocol=HTTP && port=443") == "protocol=http && port=443")
    }

    @Test
    func saveScopeUsesFilterOnlyWhenRequested() {
        #expect(FilterExpressionNormalizer.saveFilter(for: .wholeCapture, expression: "udp and 443") == nil)
        #expect(FilterExpressionNormalizer.saveFilter(for: .filteredOnly, expression: "udp and 443") == "protocol=udp && port=443")
    }

    @Test
    func engineInfoParserReadsCapabilitiesPayload() {
        let payload: [String: Any] = [
            "schema_version": "v1",
            "engine_version": "0.2.0",
            "capabilities": [
                "inspect": true,
                "packet_list": true,
                "packet_detail": true,
                "stats": true,
                "conversations": true,
                "streams": true,
                "transactions": true,
                "save": true,
                "live_capture": true
            ],
            "capture": [
                "bundled_backend": true,
                "built_in_tcpdump": true,
                "interface_discovery": true,
                "requires_admin_for_live_capture": true
            ],
            "filters": [
                "packet_filters": true,
                "stream_filters": true,
                "transaction_filters": true,
                "shorthand_protocol_terms": true,
                "shorthand_port_terms": true,
                "case_insensitive_protocols": true,
                "alternate_and_operators": ["&&", "&", "and"]
            ],
            "export": [
                "save_capture": true,
                "filtered_save": true,
                "whole_capture_save": true
            ],
            "dissectors": [
                "protocols": ["http", "tcp", "udp"]
            ]
        ]

        let parsed = EngineCapabilitiesParser.parse(payload)
        #expect(parsed.version == "0.2.0")
        #expect(parsed.supportsTransactions)
        #expect(parsed.capture.requiresAdminForLiveCapture)
        #expect(parsed.filters.alternateAndOperators == ["&&", "&", "and"])
        #expect(parsed.supportedProtocols.contains("http"))
    }

    @Test
    func bundledCliCanInspectFixtureCapture() throws {
        let oldCLI = ProcessInfo.processInfo.environment["ICESNIFF_CLI_BIN"]
        setenv("ICESNIFF_CLI_BIN", bundledCLIPath.path, 1)
        defer {
            if let oldCLI {
                setenv("ICESNIFF_CLI_BIN", oldCLI, 1)
            } else {
                unsetenv("ICESNIFF_CLI_BIN")
            }
        }

        let data = try CliBridge.runJSONData(
            repoRoot: packageRootURL,
            args: EngineCommand.inspect(path: fixtureCapturePath.path).args
        )
        let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]

        #expect(object?["format"] as? String == "pcap")
        #expect((object?["packet_count_hint"] as? NSNumber)?.intValue ?? 0 > 0)
    }

    @Test
    func bundledCliExposesEngineInfo() throws {
        let oldCLI = ProcessInfo.processInfo.environment["ICESNIFF_CLI_BIN"]
        setenv("ICESNIFF_CLI_BIN", bundledCLIPath.path, 1)
        defer {
            if let oldCLI {
                setenv("ICESNIFF_CLI_BIN", oldCLI, 1)
            } else {
                unsetenv("ICESNIFF_CLI_BIN")
            }
        }

        let data = try CliBridge.runJSONData(
            repoRoot: packageRootURL,
            args: EngineCommand.engineInfo.args
        )
        let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let parsed = EngineCapabilitiesParser.parse(object ?? [:])

        #expect(parsed.supportsInspect)
        #expect(parsed.supportsLiveCapture)
        #expect(parsed.supportedProtocols.contains("tcp"))
    }

    @Test
    func privilegedCaptureCommandsCoverStartAndStopLifecycle() {
        let launch = PrivilegedCaptureCommandBuilder.launchCommand(
            executablePath: "/usr/sbin/tcpdump",
            arguments: ["-i", "en0", "-U", "-w", "/tmp/capture.pcap"],
            pidFile: "/tmp/ice.pid",
            errorFile: "/tmp/ice.err"
        )
        let stop = PrivilegedCaptureCommandBuilder.stopCommand(
            pidFile: "/tmp/ice.pid",
            errorFile: "/tmp/ice.err"
        )

        #expect(launch.contains("nohup"))
        #expect(launch.contains("/usr/sbin/tcpdump"))
        #expect(launch.contains("/tmp/ice.pid"))
        #expect(stop.contains("kill -TERM"))
        #expect(stop.contains("kill -KILL"))
        #expect(stop.contains("rm -f"))
    }

    @Test
    func oneTimeCaptureSetupInstallsLaunchDaemonArtifacts() {
        let command = CapturePrivilegeSetup.installCommand()
        #expect(command.contains(CapturePrivilegeSetup.scriptPath))
        #expect(command.contains(CapturePrivilegeSetup.plistPath))
        #expect(command.contains("launchctl bootstrap system"))
        #expect(command.contains("chmod 660"))
    }

    @Test
    func liveCaptureErrorMapperProducesUserFacingMessages() {
        let canceled = LiveCaptureErrorMapper.message(for: CliBridgeError.commandFailed("User canceled."))
        let denied = LiveCaptureErrorMapper.message(for: CliBridgeError.commandFailed("tcpdump: /dev/bpf0: Operation not permitted"))

        #expect(canceled.contains("canceled"))
        #expect(denied.contains("administrator approval"))
    }

    @Test
    func explicitCaptureHelperResolutionUsesIceSniffHelper() throws {
        let helperURL = FileManager.default.temporaryDirectory.appendingPathComponent("icesniff-capture-helper-test")
        FileManager.default.createFile(atPath: helperURL.path, contents: Data("#!/bin/sh\n".utf8))
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: helperURL.path)
        defer { try? FileManager.default.removeItem(at: helperURL) }

        setenv("ICESNIFF_CAPTURE_HELPER_BIN", helperURL.path, 1)
        defer { unsetenv("ICESNIFF_CAPTURE_HELPER_BIN") }

        switch LiveCaptureBridge.resolveRuntime() {
        case let .available(runtime):
            #expect(runtime.backendKind.displayName == "IceSniff Capture")
            #expect(runtime.executableURL.path == helperURL.path)
            #expect(runtime.interfaceListArguments == ["list-interfaces"])
        case let .unavailable(message):
            Issue.record("expected helper runtime to resolve, got: \(message)")
        }
    }

    private var packageRootURL: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }

    private var bundledCLIPath: URL {
        packageRootURL.appendingPathComponent("Sources/IceSniffMac/Resources/BundledCLI/icesniff-cli")
    }

    private var fixtureCapturePath: URL {
        let fixtureURL = FileManager.default.temporaryDirectory.appendingPathComponent("icesniff-test-sample.pcap")
        if !FileManager.default.fileExists(atPath: fixtureURL.path) {
            let hexFixtureURL = packageRootURL
                .appendingPathComponent("Tests/Fixtures/sample.pcap.hex")
            let hexString = (try? String(contentsOf: hexFixtureURL, encoding: .utf8))?
                .components(separatedBy: .whitespacesAndNewlines)
                .joined() ?? ""
            let bytes = stride(from: 0, to: hexString.count, by: 2).compactMap { index -> UInt8? in
                let start = hexString.index(hexString.startIndex, offsetBy: index)
                let end = hexString.index(start, offsetBy: 2, limitedBy: hexString.endIndex) ?? hexString.endIndex
                return UInt8(hexString[start..<end], radix: 16)
            }
            FileManager.default.createFile(atPath: fixtureURL.path, contents: Data(bytes))
        }
        return fixtureURL
    }
}
