//! Locating the Flutter views that belong to an engine.
//!
//! The engine context only knows about the implicit view, which does not exist
//! when the engine runs without one (for example when every window is created
//! through the multi-view windowing API). These helpers find the views of an
//! engine by looking at the application windows whose content view controller
//! is a `FlutterViewController` driven by that engine. The engine is matched by
//! its binary messenger, which is the same object that the plugin registrar
//! hands out.

use irondash_engine_context::EngineContext;
use objc2::{
    msg_send, msg_send_id,
    rc::{Id, WeakId},
    runtime::{AnyClass, AnyObject, NSObject},
    sel, ClassType,
};
use objc2_app_kit::{NSApplication, NSView, NSWindow};
use objc2_foundation::MainThreadMarker;

use crate::error::NativeExtensionsResult;

pub struct FlutterViewInfo {
    pub window: Id<NSWindow>,
    pub view: Id<NSView>,
    pub view_id: i64,
}

pub struct EngineViews {
    /// The implicit view, if the engine has one.
    implicit_view: Option<Id<NSView>>,
    messenger: WeakId<NSObject>,
}

impl EngineViews {
    pub fn new(engine_handle: i64) -> NativeExtensionsResult<Self> {
        let engine_context = EngineContext::get()?;
        let messenger = engine_context.get_binary_messenger(engine_handle)?;
        let implicit_view = engine_context
            .get_flutter_view(engine_handle)
            .ok()
            .map(|view| unsafe { Id::cast(view) });
        Ok(Self {
            implicit_view,
            messenger: WeakId::from_id(&messenger),
        })
    }

    pub fn implicit_view(&self) -> Option<&Id<NSView>> {
        self.implicit_view.as_ref()
    }

    /// Returns the Flutter view hosted in `window` if it belongs to this engine.
    pub fn view_in_window(&self, window: &NSWindow) -> Option<FlutterViewInfo> {
        let messenger = self.messenger.load()?;
        let (view, view_id, window_messenger) = flutter_view_in_window(window)?;
        if Id::as_ptr(&window_messenger) == Id::as_ptr(&messenger) {
            Some(FlutterViewInfo {
                window: window.retain(),
                view,
                view_id,
            })
        } else {
            None
        }
    }

    /// Returns all Flutter views of this engine that are hosted in a window.
    pub fn views(&self) -> Vec<FlutterViewInfo> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Vec::new();
        };
        let windows = NSApplication::sharedApplication(mtm).windows();
        windows
            .iter()
            .filter_map(|window| self.view_in_window(window))
            .collect()
    }

    /// Returns the view identifier of `view` if it is a view of this engine
    /// hosted in a window.
    pub fn view_id(&self, view: &NSView) -> Option<i64> {
        let window = view.window()?;
        let info = self.view_in_window(&window)?;
        if *info.view == *view {
            Some(info.view_id)
        } else {
            None
        }
    }

    /// The view to use for operations that are not tied to a particular view:
    /// the implicit view if there is one, otherwise the engine's view in the
    /// key window, the main window or any other window.
    pub fn default_view(&self) -> Option<Id<NSView>> {
        if let Some(view) = &self.implicit_view {
            return Some(view.clone());
        }
        let mtm = MainThreadMarker::new()?;
        let app = NSApplication::sharedApplication(mtm);
        app.keyWindow()
            .or_else(|| unsafe { app.mainWindow() })
            .and_then(|window| self.view_in_window(&window))
            .or_else(|| self.views().into_iter().next())
            .map(|info| info.view)
    }
}

/// Returns the Flutter view, its identifier and the engine binary messenger
/// for a window whose content view controller is a `FlutterViewController`.
fn flutter_view_in_window(window: &NSWindow) -> Option<(Id<NSView>, i64, Id<NSObject>)> {
    let class = AnyClass::get("FlutterViewController")?;
    unsafe {
        let controller: Option<Id<NSObject>> = msg_send_id![window, contentViewController];
        let controller = controller?;
        let is_flutter_controller: bool = msg_send![&controller, isKindOfClass: class];
        if !is_flutter_controller
            || !responds_to(&controller, sel!(flutterView))
            || !responds_to(&controller, sel!(viewIdentifier))
        {
            return None;
        }
        let view: Option<Id<NSView>> = msg_send_id![&controller, flutterView];
        let engine: Option<Id<NSObject>> = msg_send_id![&controller, engine];
        let messenger: Option<Id<NSObject>> = msg_send_id![&engine?, binaryMessenger];
        let view_id: i64 = msg_send![&controller, viewIdentifier];
        Some((view?, view_id, messenger?))
    }
}

fn responds_to(object: &NSObject, selector: objc2::runtime::Sel) -> bool {
    unsafe { msg_send![object, respondsToSelector: selector] }
}

/// Whether `object` is the same object as `window`.
pub fn is_same_window(object: Option<&AnyObject>, window: &NSWindow) -> bool {
    object.is_some_and(|object| std::ptr::eq(object, window as *const NSWindow as *const AnyObject))
}
