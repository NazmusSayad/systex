use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;
use std::sync::OnceLock;

use accessibility_sys::{
    AXObserverAddNotification, AXObserverCreate, AXObserverGetRunLoopSource, AXObserverRef,
    AXObserverRemoveNotification, AXUIElementCreateApplication, AXUIElementRef,
    kAXFocusedUIElementChangedNotification, kAXFocusedWindowAttribute,
    kAXSelectedTextChangedNotification, kAXValueChangedNotification, kAXWindowMovedNotification,
    kAXWindowResizedNotification,
};
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::runloop::{
    CFRunLoop, CFRunLoopAddSource, CFRunLoopRemoveSource, kCFRunLoopDefaultMode,
};
use core_foundation::string::{CFString, CFStringRef};

use crate::macos::elem::{APP_TIMEOUT, Elem};
use crate::model::{AppInfo, ContextSnapshot, now_ms};
use crate::permissions::{self, Permission, PermissionStatus};

type Listener = Box<dyn Fn(ContextSnapshot) + Send + Sync>;

static LISTENER: OnceLock<Listener> = OnceLock::new();

struct EngineState {
    app_info: AppInfo,
    observer: AXObserverRef,
    app: Elem,
    focused: Option<Elem>,
    has_text: bool,
}

thread_local! {
    static ENGINE: RefCell<Option<EngineState>> = const { RefCell::new(None) };
}

/// Installs the callback that every snapshot is handed to. Answers `false` when one is already
/// installed, since the engine has a single owner.
pub fn listen(callback: impl Fn(ContextSnapshot) + Send + Sync + 'static) -> bool {
    LISTENER.set(Box::new(callback)).is_ok()
}

/// Must be called on the thread that owns the main run loop: it re-targets the observer when the
/// frontmost app changes, and publishes a snapshot so pointer movement is reported too.
pub fn tick() {
    if permissions::status(Permission::Accessibility) != PermissionStatus::Granted {
        return;
    }

    let Some(app_info) = super::frontmost_app() else {
        return;
    };
    let pid = app_info.pid;

    if current_pid() == Some(pid) {
        // Chromium finishes building its tree some time after `AXManualAccessibility` is set, and a
        // few apps never fire a focus notification at all, so keep re-reading while we have no text.
        if !has_text() {
            reassert_accessibility_tree();
            rebind_focused_element();
        }

        publish();
        return;
    }

    release();
    bind(pid, app_info);
}

pub fn stop() {
    release();
}

unsafe extern "C" fn observer_callback(
    _observer: AXObserverRef,
    _element: AXUIElementRef,
    notification: CFStringRef,
    _refcon: *mut c_void,
) {
    let notification = unsafe { CFString::wrap_under_get_rule(notification) }.to_string();

    if notification == kAXFocusedUIElementChangedNotification {
        rebind_focused_element();
    }

    publish();
}

fn bind(pid: i32, app_info: AppInfo) {
    ENGINE.with(|engine| {
        let mut observer: AXObserverRef = ptr::null_mut();
        let err = unsafe { AXObserverCreate(pid, observer_callback, &mut observer) };

        if err != 0 || observer.is_null() {
            return;
        }

        let Some(app) =
            Elem::from_create_rule(unsafe { AXUIElementCreateApplication(pid) }, APP_TIMEOUT)
        else {
            return;
        };

        app.enable_accessibility_tree();

        for notification in [
            kAXFocusedUIElementChangedNotification,
            kAXWindowMovedNotification,
            kAXWindowResizedNotification,
        ] {
            unsafe {
                AXObserverAddNotification(
                    observer,
                    app.0,
                    CFString::new(notification).as_concrete_TypeRef(),
                    ptr::null_mut(),
                )
            };
        }

        unsafe {
            CFRunLoopAddSource(
                CFRunLoop::get_current().as_concrete_TypeRef(),
                AXObserverGetRunLoopSource(observer),
                kCFRunLoopDefaultMode,
            )
        };

        *engine.borrow_mut() = Some(EngineState {
            app_info,
            observer,
            app,
            focused: None,
            has_text: false,
        });
    });

    rebind_focused_element();
    publish();
}

fn rebind_focused_element() {
    ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        let Some(state) = engine.as_mut() else {
            return;
        };

        if let Some(previous) = state.focused.take() {
            for notification in [
                kAXValueChangedNotification,
                kAXSelectedTextChangedNotification,
            ] {
                unsafe {
                    AXObserverRemoveNotification(
                        state.observer,
                        previous.0,
                        CFString::new(notification).as_concrete_TypeRef(),
                    )
                };
            }
        }

        let Some(focused) = super::focused_element(&state.app) else {
            return;
        };

        for notification in [
            kAXValueChangedNotification,
            kAXSelectedTextChangedNotification,
        ] {
            unsafe {
                AXObserverAddNotification(
                    state.observer,
                    focused.0,
                    CFString::new(notification).as_concrete_TypeRef(),
                    ptr::null_mut(),
                )
            };
        }

        state.focused = Some(focused);
    });
}

fn publish() {
    let Some(listener) = LISTENER.get() else {
        return;
    };

    let snapshot = ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        let state = engine.as_mut()?;
        let window = state
            .app
            .element_attribute(kAXFocusedWindowAttribute)
            .or_else(|| super::first_window(&state.app));
        let caret = state
            .focused
            .as_ref()
            .map(|element| super::caret_context(element, false));

        state.has_text = caret.as_ref().is_some_and(|caret| {
            caret
                .element
                .as_ref()
                .is_some_and(|element| element.editable)
        });

        Some(ContextSnapshot {
            captured_at_ms: now_ms(),
            provider: "macos-accessibility".to_string(),
            focused_app: Some(state.app_info.clone()),
            focused_window: window.as_ref().map(super::window_info),
            related: caret
                .as_ref()
                .map(|caret| crate::related::from_caret(&caret.text_before, &caret.text_after)),
            caret,
            pointer: super::pointer_context(false),
            window_text: None,
            window_tree: None,
            words: Vec::new(),
        })
    });

    if let Some(snapshot) = snapshot {
        listener(snapshot);
    }
}

fn reassert_accessibility_tree() {
    ENGINE.with(|engine| {
        if let Some(state) = engine.borrow().as_ref() {
            state.app.enable_accessibility_tree();
        }
    });
}

fn has_text() -> bool {
    ENGINE.with(|engine| match engine.borrow().as_ref() {
        Some(state) => state.has_text,
        None => false,
    })
}

fn current_pid() -> Option<i32> {
    ENGINE.with(|engine| engine.borrow().as_ref().map(|state| state.app_info.pid))
}

fn release() {
    ENGINE.with(|engine| {
        if let Some(state) = engine.borrow_mut().take() {
            unsafe {
                CFRunLoopRemoveSource(
                    CFRunLoop::get_current().as_concrete_TypeRef(),
                    AXObserverGetRunLoopSource(state.observer),
                    kCFRunLoopDefaultMode,
                )
            };

            drop(state.app);
            drop(state.focused);

            unsafe { CFRelease(state.observer as CFTypeRef) };
        }
    });
}
