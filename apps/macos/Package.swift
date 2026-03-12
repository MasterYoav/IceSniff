// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "IceSniffMac",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "IceSniffMac", targets: ["IceSniffMac"])
    ],
    targets: [
        .executableTarget(
            name: "IceSniffMac",
            resources: [
                .process("Resources/icon.icon"),
                .process("Resources/icon-dark.png"),
                .process("Resources/icon-light.png"),
                .copy("Resources/BundledCLI/icesniff-cli"),
                .copy("Resources/BundledCLI/icesniff-capture-helper")
            ]
        ),
        .testTarget(
            name: "IceSniffMacTests",
            dependencies: ["IceSniffMac"]
        )
    ]
)
