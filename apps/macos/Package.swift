// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "IceSniffMac",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "IceSniffMac", targets: ["IceSniffMac"])
    ],
    dependencies: [
        .package(url: "https://github.com/supabase/supabase-swift", from: "2.0.0")
    ],
    targets: [
        .executableTarget(
            name: "IceSniffMac",
            dependencies: [
                .product(name: "Supabase", package: "supabase-swift")
            ],
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
