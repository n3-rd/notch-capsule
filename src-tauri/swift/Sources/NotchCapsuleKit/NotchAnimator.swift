import AppKit
import QuartzCore

@objc public enum NotchPhase: Int { 
    case expand
    case collapse 
}

// Config structures matching the JSON config file
struct NotchConfig: Codable {
    let animation: AnimationConfig
    let dimensions: DimensionsConfig
    let hover: HoverConfig
    let window: WindowConfig
}

struct AnimationConfig: Codable {
    let expand_duration: ConfigValue<Double>
    let collapse_duration: ConfigValue<Double>
    let expand_timing: ConfigValue<[Double]>
    let collapse_timing: ConfigValue<[Double]>
}

struct DimensionsConfig: Codable {
    let corner_radius: ConfigValue<Double>
    let collapsed_width: ConfigValue<Double>
    let collapsed_height: ConfigValue<Double>
    let expanded_width: ConfigValue<Double>
    let expanded_height: ConfigValue<Double>
}

struct HoverConfig: Codable {
    let collapsed_zone_width: ConfigValue<Double>
    let collapsed_zone_height: ConfigValue<Double>
    let expanded_zone_width: ConfigValue<Double>
    let expanded_zone_height: ConfigValue<Double>
    let expand_delay_ms: ConfigValue<Int>
    let collapse_delay_ms: ConfigValue<Int>
    let poll_interval_ms: ConfigValue<Int>
}

struct WindowConfig: Codable {
    let level_offset: ConfigValue<Int>
}

struct ConfigValue<T: Codable>: Codable {
    let value: T
    let description: String
}

@objc public class NotchAnimator: NSObject {
    private weak var window: NSWindow?
    private let maskLayer = CAShapeLayer()
    private let hitView = HitTestView()
    private var closedRect: CGRect = .zero
    private var expandedRect: CGRect = .zero
    private var corner: CGFloat = 12
    
    // Animation timing from config - defaults matching Boring Notch feel
    private var expandDuration: CFTimeInterval = 0.4
    private var collapseDuration: CFTimeInterval = 0.3
    private var expandTimingFunction = CAMediaTimingFunction(controlPoints: 0.16, 1.0, 0.3, 1.0)
    private var collapseTimingFunction = CAMediaTimingFunction(controlPoints: 0.25, 0.1, 0.25, 1.0)
    private var windowLevelOffset: Int = 3
    private var useSpringAnimation: Bool = true

    @objc public func attach(to window: NSWindow, closedRect: CGRect, expandedRect: CGRect, corner: CGFloat) {
        self.window = window
        self.closedRect = closedRect
        self.expandedRect = expandedRect
        self.corner = corner

        window.isOpaque = false
        window.backgroundColor = .clear
        window.level = .init(NSWindow.Level.mainMenu.rawValue + windowLevelOffset)
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]

        guard let contentView = window.contentView else { return }
        contentView.wantsLayer = true
        if contentView.layer == nil { 
            contentView.layer = CALayer() 
        }
        
        // Ensure layer is ready for masking
        guard let layer = contentView.layer else { return }

        // Configure mask layer with proper defaults
        maskLayer.fillColor = NSColor.black.cgColor
        maskLayer.fillRule = .evenOdd
        maskLayer.frame = layer.bounds
        layer.mask = maskLayer

        hitView.frame = contentView.bounds
        hitView.autoresizingMask = [.width, .height]
        contentView.addSubview(hitView, positioned: .above, relativeTo: nil)

        // Force initial layout
        contentView.layoutSubtreeIfNeeded()
        updatePath(progress: 0) // closed
        
        // Ensure changes are committed
        CATransaction.flush()
    }
    
    @objc public func setConfigJson(_ jsonCString: UnsafePointer<CChar>) {
        guard let jsonString = String(utf8String: jsonCString) else {
            print("Failed to convert C string to Swift String")
            return
        }
        
        guard let jsonData = jsonString.data(using: .utf8) else {
            print("Failed to convert string to UTF-8 data")
            return
        }
        
        do {
            let decoder = JSONDecoder()
            let config = try decoder.decode(NotchConfig.self, from: jsonData)
            
            // Apply animation config
            expandDuration = config.animation.expand_duration.value
            collapseDuration = config.animation.collapse_duration.value
            
            // Apply timing functions from control points
            let expandPoints = config.animation.expand_timing.value
            if expandPoints.count == 4 {
                expandTimingFunction = CAMediaTimingFunction(
                    controlPoints: Float(expandPoints[0]),
                    Float(expandPoints[1]),
                    Float(expandPoints[2]),
                    Float(expandPoints[3])
                )
            }
            
            let collapsePoints = config.animation.collapse_timing.value
            if collapsePoints.count == 4 {
                collapseTimingFunction = CAMediaTimingFunction(
                    controlPoints: Float(collapsePoints[0]),
                    Float(collapsePoints[1]),
                    Float(collapsePoints[2]),
                    Float(collapsePoints[3])
                )
            }
            
            // Apply dimensions config
            corner = config.dimensions.corner_radius.value
            
            // Apply window level config
            windowLevelOffset = config.window.level_offset.value
            if let win = window {
                win.level = .init(NSWindow.Level.mainMenu.rawValue + windowLevelOffset)
            }
            
            // Refresh path with new corner radius if we have a current progress
            if maskLayer.path != nil {
                // Get current progress - if collapsed it's 0, if expanded it's 1
                let currentProgress: CGFloat = (closedRect.width == expandedRect.width) ? 0 : 1
                updatePath(progress: currentProgress)
            }
            
            print("✓ Successfully loaded animation config from JSON")
        } catch {
            print("✗ Failed to decode config JSON: \(error)")
            if let decodingError = error as? DecodingError {
                switch decodingError {
                case .dataCorrupted(let context):
                    print("  Data corrupted: \(context)")
                case .keyNotFound(let key, let context):
                    print("  Key '\(key)' not found: \(context)")
                case .typeMismatch(let type, let context):
                    print("  Type mismatch for \(type): \(context)")
                case .valueNotFound(let type, let context):
                    print("  Value not found for \(type): \(context)")
                @unknown default:
                    print("  Unknown decoding error")
                }
            }
        }
    }

    @objc public func expand(duration: CFTimeInterval, appHandle: UnsafeMutableRawPointer?) {
        animate(to: 1.0, duration: expandDuration, timingFunction: expandTimingFunction)
        // Use actual animation duration for spring animation
        let actualDuration: CFTimeInterval
        if #available(macOS 14.0, *), useSpringAnimation {
            actualDuration = 0.5 // Spring settling time
        } else {
            actualDuration = expandDuration
        }
        notifyEnd(phase: .expand, after: actualDuration, appHandle: appHandle)
    }

    @objc public func collapse(duration: CFTimeInterval, appHandle: UnsafeMutableRawPointer?) {
        animate(to: 0.0, duration: collapseDuration, timingFunction: collapseTimingFunction)
        // Use actual animation duration for spring animation
        let actualDuration: CFTimeInterval
        if #available(macOS 14.0, *), useSpringAnimation {
            actualDuration = 0.35 // Spring settling time
        } else {
            actualDuration = collapseDuration
        }
        notifyEnd(phase: .collapse, after: actualDuration, appHandle: appHandle)
    }

    @objc public func setProgress(_ p: CGFloat) { 
        updatePath(progress: max(0, min(1, p))) 
    }

    private func animate(to target: CGFloat, duration: CFTimeInterval, timingFunction: CAMediaTimingFunction) {
        guard let contentView = window?.contentView else { return }
        contentView.layoutSubtreeIfNeeded()

        let fromPath = maskLayer.path
        updatePath(progress: target)
        let toPath = maskLayer.path
        
        // Remove any existing animations to prevent conflicts
        maskLayer.removeAllAnimations()
        maskLayer.path = toPath

        if #available(macOS 14.0, *), useSpringAnimation {
            // Use spring animation for smoother, more natural feel
            let anim = CASpringAnimation(keyPath: "path")
            anim.fromValue = fromPath
            anim.toValue = toPath
            anim.mass = 1.0
            anim.stiffness = 300.0
            anim.damping = 30.0
            anim.duration = anim.settlingDuration
            anim.fillMode = .forwards
            anim.isRemovedOnCompletion = false
            maskLayer.add(anim, forKey: "path")
        } else {
            // Fallback to timing function for older macOS
            let anim = CABasicAnimation(keyPath: "path")
            anim.fromValue = fromPath
            anim.toValue = toPath
            anim.duration = duration
            anim.timingFunction = timingFunction
            anim.fillMode = .forwards
            anim.isRemovedOnCompletion = false
            maskLayer.add(anim, forKey: "path")
        }
    }

    private func updatePath(progress: CGFloat) {
        // Lerp rect and corner
        let w = closedRect.width  + (expandedRect.width  - closedRect.width)  * progress
        let h = closedRect.height + (expandedRect.height - closedRect.height) * progress
        let rect = CGRect(x: (expandedRect.width - w) / 2.0,
                          y: (expandedRect.height - h), // anchor to top edge; grow downward
                          width: w, height: h)

        let path = CGPath(roundedRect: rect, cornerWidth: corner, cornerHeight: corner, transform: nil)
        maskLayer.path = path
        hitView.currentPath = path
    }

    private func notifyEnd(phase: NotchPhase, after: CFTimeInterval, appHandle: UnsafeMutableRawPointer?) {
        guard let appHandle = appHandle else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + after) {
            // Call the Rust function directly via dlsym
            if let rustCallback = dlsym(UnsafeMutableRawPointer(bitPattern: -2), "_notch_notify_anim_end") {
                typealias CallbackType = @convention(c) (UnsafeMutableRawPointer, Int32) -> Void
                let callback = unsafeBitCast(rustCallback, to: CallbackType.self)
                callback(appHandle, Int32(phase.rawValue))
            }
        }
    }
}

final class HitTestView: NSView {
    var currentPath: CGPath? = nil
    
    override func hitTest(_ point: NSPoint) -> NSView? {
        guard let path = currentPath, let layer = superview?.layer else { return nil }
        let p = convert(point, to: superview)
        let inPath = path.contains(p)
        return inPath ? super.hitTest(point) : nil
    }
}
