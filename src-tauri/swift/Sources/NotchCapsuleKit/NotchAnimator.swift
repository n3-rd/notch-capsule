import AppKit
import QuartzCore

@objc public enum NotchPhase: Int { 
    case expand
    case collapse 
}

@objc public class NotchAnimator: NSObject {
    private weak var window: NSWindow?
    private let maskLayer = CAShapeLayer()
    private let hitView = HitTestView()
    private var closedRect: CGRect = .zero
    private var expandedRect: CGRect = .zero
    private var corner: CGFloat = 12
    
    // Animation timing matching Boring Notch feel - more fluid and motion-like
    // Reference: https://github.com/TheBoredTeam/boring.notch
    private let expandDuration: CFTimeInterval = 0.50  // Slower, more fluid spring feel
    private let collapseDuration: CFTimeInterval = 0.35 // Smooth, gentle collapse
    private let expandTimingFunction = CAMediaTimingFunction(controlPoints: 0.16, 1.0, 0.3, 1.0) // Fluid spring
    private let collapseTimingFunction = CAMediaTimingFunction(controlPoints: 0.25, 0.1, 0.25, 1.0) // Smooth ease out

    @objc public func attach(to window: NSWindow, closedRect: CGRect, expandedRect: CGRect, corner: CGFloat) {
        self.window = window
        self.closedRect = closedRect
        self.expandedRect = expandedRect
        self.corner = corner

        window.isOpaque = false
        window.backgroundColor = .clear
        window.level = .init(NSWindow.Level.mainMenu.rawValue + 3)
        window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]

        guard let contentView = window.contentView else { return }
        contentView.wantsLayer = true
        if contentView.layer == nil { 
            contentView.layer = CALayer() 
        }

        maskLayer.fillColor = NSColor.black.cgColor
        contentView.layer?.mask = maskLayer

        hitView.frame = contentView.bounds
        hitView.autoresizingMask = [.width, .height]
        contentView.addSubview(hitView, positioned: .above, relativeTo: nil)

        updatePath(progress: 0) // closed
    }

    @objc public func expand(duration: CFTimeInterval, appHandle: UnsafeMutableRawPointer?) {
        animate(to: 1.0, duration: expandDuration, timingFunction: expandTimingFunction)
        notifyEnd(phase: .expand, after: expandDuration, appHandle: appHandle)
    }

    @objc public func collapse(duration: CFTimeInterval, appHandle: UnsafeMutableRawPointer?) {
        animate(to: 0.0, duration: collapseDuration, timingFunction: collapseTimingFunction)
        notifyEnd(phase: .collapse, after: collapseDuration, appHandle: appHandle)
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
        maskLayer.path = toPath

        let anim = CABasicAnimation(keyPath: "path")
        anim.fromValue = fromPath
        anim.toValue = toPath
        anim.duration = duration
        anim.timingFunction = timingFunction
        maskLayer.add(anim, forKey: "path")
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
