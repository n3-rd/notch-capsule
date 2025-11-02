import AppKit
import SwiftUI
import Combine

// MARK: - Notch Manager (Main Controller)
@objc public class NotchManager: NSObject {
    @objc public static let shared = NotchManager()
    
    private var window: NotchWindow?
    private var animator: NotchAnimator?
    private var hoverMonitor: HoverMonitor?
    private var config: NotchConfig?
    
    private var isExpanded: Bool = false
    private var isHovering: Bool = false
    private var expandTask: DispatchWorkItem?
    private var collapseTask: DispatchWorkItem?
    
    // Callbacks to Rust
    private var stateChangeCallback: ((Bool) -> Void)?
    
    private override init() {
        super.init()
    }
    
    @objc public func setup(
        closedW: CGFloat,
        closedH: CGFloat,
        expandedW: CGFloat,
        expandedH: CGFloat,
        corner: CGFloat,
        configJson: String?
    ) {
        // Load config
        if let jsonString = configJson, let data = jsonString.data(using: .utf8) {
            do {
                config = try JSONDecoder().decode(NotchConfig.self, from: data)
                print("✓ Loaded notch config from JSON")
            } catch {
                print("✗ Failed to decode config, using defaults: \(error)")
            }
        }
        
        // Create the window
        window = NotchWindow(
            closedSize: CGSize(width: closedW, height: closedH),
            expandedSize: CGSize(width: expandedW, height: expandedH),
            corner: corner
        )
        
        guard let window = window else {
            print("✗ Failed to create notch window")
            return
        }
        
        // Setup animator
        animator = NotchAnimator()
        animator?.attach(
            to: window,
            closedRect: CGRect(x: 0, y: 0, width: closedW, height: closedH),
            expandedRect: CGRect(x: 0, y: 0, width: expandedW, height: expandedH),
            corner: corner
        )
        
        if let configJson = configJson, let cString = configJson.cString(using: .utf8) {
            cString.withUnsafeBufferPointer { buffer in
                animator?.setConfigJson(buffer.baseAddress!)
            }
        }
        
        // Setup hover monitoring
        setupHoverMonitoring()
        
        // Position window at top center
        positionWindow(expanded: false)
        
        // Show window
        window.makeKeyAndOrderFront(nil)
        
        print("✓ NotchManager setup complete")
    }
    
    private func setupHoverMonitoring() {
        guard let config = config, let window = window else { return }
        
        let collapsedZone = CGSize(
            width: config.hover.collapsed_zone_width.value,
            height: config.hover.collapsed_zone_height.value
        )
        let expandedZone = CGSize(
            width: config.hover.expanded_zone_width.value,
            height: config.hover.expanded_zone_height.value
        )
        
        hoverMonitor = HoverMonitor(
            window: window,
            collapsedZone: collapsedZone,
            expandedZone: expandedZone
        )
        
        hoverMonitor?.onHoverChanged = { [weak self] isInside in
            self?.handleHoverChange(isInside)
        }
        
        hoverMonitor?.start()
    }
    
    private func handleHoverChange(_ isInside: Bool) {
        guard let config = config else { return }
        
        isHovering = isInside
        
        if isInside && !isExpanded {
            // Schedule expansion
            expandTask?.cancel()
            
            let delay = Double(config.hover.expand_delay_ms.value) / 1000.0
            let task = DispatchWorkItem { [weak self] in
                self?.expand()
            }
            expandTask = task
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: task)
            
        } else if !isInside && isExpanded {
            // Schedule collapse
            collapseTask?.cancel()
            
            let delay = Double(config.hover.collapse_delay_ms.value) / 1000.0
            let task = DispatchWorkItem { [weak self] in
                self?.collapse()
            }
            collapseTask = task
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: task)
        }
    }
    
    private func expand() {
        guard !isExpanded, let window = window, let animator = animator else { return }
        
        isExpanded = true
        
        // Make window focusable for interaction
        window.styleMask.remove(.nonactivatingPanel)
        window.makeKeyAndOrderFront(nil)
        
        // Resize window
        positionWindow(expanded: true)
        
        // Animate mask
        animator.expand(duration: 0.4, appHandle: nil)
        
        stateChangeCallback?(true)
        
        print("→ Expanded notch")
    }
    
    private func collapse() {
        guard isExpanded, let window = window, let animator = animator else { return }
        
        // Don't collapse if still hovering
        if isHovering { return }
        
        isExpanded = false
        
        // Make window non-activating again
        window.styleMask.insert(.nonactivatingPanel)
        
        // Animate mask
        animator.collapse(duration: 0.3, appHandle: nil)
        
        // Resize window after animation
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { [weak self] in
            self?.positionWindow(expanded: false)
        }
        
        stateChangeCallback?(false)
        
        print("← Collapsed notch")
    }
    
    private func positionWindow(expanded: Bool) {
        guard let window = window, let screen = NSScreen.main else { return }
        
        let screenFrame = screen.frame
        let size = expanded ? window.expandedSize : window.closedSize
        
        // Center horizontally, align to top
        let x = screenFrame.midX - size.width / 2
        let y = screenFrame.maxY - size.height
        
        window.setFrame(
            CGRect(x: x, y: y, width: size.width, height: size.height),
            display: true,
            animate: false
        )
    }
    
    @objc public func setStateChangeCallback(_ callback: @escaping (Bool) -> Void) {
        stateChangeCallback = callback
    }
    
    @objc public func forceExpand() {
        expandTask?.cancel()
        expand()
    }
    
    @objc public func forceCollapse() {
        collapseTask?.cancel()
        collapse()
    }
    
    @objc public func cleanup() {
        hoverMonitor?.stop()
        expandTask?.cancel()
        collapseTask?.cancel()
        window?.close()
        print("✓ NotchManager cleaned up")
    }
}

// MARK: - Notch Window
class NotchWindow: NSWindow {
    let closedSize: CGSize
    let expandedSize: CGSize
    let cornerRadius: CGFloat
    
    init(closedSize: CGSize, expandedSize: CGSize, corner: CGFloat) {
        self.closedSize = closedSize
        self.expandedSize = expandedSize
        self.cornerRadius = corner
        
        super.init(
            contentRect: CGRect(origin: .zero, size: closedSize),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        
        setupWindow()
    }
    
    private func setupWindow() {
        // Window properties matching Boring Notch
        isOpaque = false
        backgroundColor = .clear
        hasShadow = false
        level = .statusBar
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
        
        // Allow mouse events
        ignoresMouseEvents = false
        acceptsMouseMovedEvents = true
        
        // Content view
        if let contentView = contentView {
            contentView.wantsLayer = true
            contentView.layer?.backgroundColor = NSColor.clear.cgColor
        }
    }
}

// MARK: - Hover Monitor
class HoverMonitor {
    private weak var window: NSWindow?
    private let collapsedZone: CGSize
    private let expandedZone: CGSize
    private var eventMonitor: Any?
    private var trackingArea: NSTrackingArea?
    private var pollTimer: Timer?
    
    var onHoverChanged: ((Bool) -> Void)?
    private var lastState: Bool = false
    
    init(window: NSWindow, collapsedZone: CGSize, expandedZone: CGSize) {
        self.window = window
        self.collapsedZone = collapsedZone
        self.expandedZone = expandedZone
    }
    
    func start() {
        // Global event monitor for mouse moves
        eventMonitor = NSEvent.addGlobalMonitorForEvents(matching: .mouseMoved) { [weak self] event in
            self?.checkHover(at: event)
        }
        
        // Polling fallback for when global monitor doesn't work
        pollTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            self?.checkHover(at: nil)
        }
        
        print("✓ Hover monitoring started")
    }
    
    func stop() {
        if let monitor = eventMonitor {
            NSEvent.removeMonitor(monitor)
        }
        pollTimer?.invalidate()
        print("✓ Hover monitoring stopped")
    }
    
    private func checkHover(at event: NSEvent?) {
        guard let screen = NSScreen.main else { return }
        
        let mouseLocation = NSEvent.mouseLocation
        let screenFrame = screen.frame
        
        // Define hover zone at top center of screen
        let zoneWidth = collapsedZone.width
        let zoneHeight = collapsedZone.height
        let zoneX = screenFrame.midX - zoneWidth / 2
        let zoneY = screenFrame.maxY - zoneHeight
        let hoverZone = CGRect(x: zoneX, y: zoneY, width: zoneWidth, height: zoneHeight)
        
        let isInside = hoverZone.contains(mouseLocation)
        
        if isInside != lastState {
            lastState = isInside
            onHoverChanged?(isInside)
        }
    }
}

