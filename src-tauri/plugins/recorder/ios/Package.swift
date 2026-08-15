// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "tauri-plugin-recorder",
    platforms: [.iOS(.v15)],
    products: [
        .library(
            name: "tauri-plugin-recorder",
            type: .static,
            targets: ["tauri-plugin-recorder"]
        ),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
    ],
    targets: [
        .target(
            name: "tauri-plugin-recorder",
            dependencies: [.byName(name: "Tauri")],
            path: "Sources"
        ),
        .testTarget(
            name: "PluginTests",
            dependencies: ["tauri-plugin-recorder"],
            path: "Tests/PluginTests"
        ),
    ]
)
