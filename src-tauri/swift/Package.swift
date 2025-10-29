// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NotchCapsuleKit",
    platforms: [
        .macOS(.v12)
    ],
    products: [
        .library(
            name: "NotchCapsuleKit",
            type: .dynamic,
            targets: ["NotchCapsuleKit"]
        )
    ],
    targets: [
        .target(
            name: "NotchCapsuleKit",
            path: "Sources/NotchCapsuleKit"
        )
    ]
)
