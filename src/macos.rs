use std::ffi::c_void;
use std::ptr;

use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};

use crate::error::{Result, SystexError};
use crate::model::{
    AppInfo, CaretContext, ContextSnapshot, ElementInfo, Point, PointerContext, Rect, WindowInfo,
    now_ms,
};

type AXUIElementRef = *const c_void;
type AXValueRef = *const c_void;

const AX_VALUE_CG_POINT: u32 = 1;
const AX_VALUE_CG_SIZE: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

const FOCUSED_APPLICATION: &str = "AXFocusedApplication";
const FOCUSED_WINDOW: &str = "AXFocusedWindow";
const FOCUSED_ELEMENT: &str = "AXFocusedUIElement";
const TITLE: &str = "AXTitle";
const DESCRIPTION: &str = "AXDescription";
const ROLE: &str = "AXRole";
const POSITION: &str = "AXPosition";
const SIZE: &str = "AXSize";

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;

    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;

    fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> i32;

    fn AXValueGetValue(value: AXValueRef, kind: u32, out: *mut c_void) -> bool;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreate(source: *const c_void) -> *const c_void;

    fn CGEventGetLocation(event: *const c_void) -> CGPoint;
}

pub fn capture() -> Result<ContextSnapshot> {
    let system = unsafe { AXUIElementCreateSystemWide() };

    if system.is_null() {
        return Err(SystexError::Capture(
            "the system-wide accessibility element is unavailable".to_string(),
        ));
    }

    let app = unsafe { copy_element(system, FOCUSED_APPLICATION) };

    if app.is_none() {
        unsafe { CFRelease(system) };

        return Err(SystexError::NothingFocused);
    }

    let app = app.expect("the focused application is present");
    let focused_app = unsafe { app_info(app) };
    let window = unsafe { copy_element(app, FOCUSED_WINDOW) };
    let focused_window = window.map(|window| unsafe { window_info(window) });
    let element = unsafe { copy_element(app, FOCUSED_ELEMENT) };
    let caret = element.map(|element| unsafe { caret_context(element) });

    if let Some(window) = window {
        unsafe { CFRelease(window) };
    }
    if let Some(element) = element {
        unsafe { CFRelease(element) };
    }
    unsafe { CFRelease(app) };
    unsafe { CFRelease(system) };

    Ok(ContextSnapshot {
        captured_at_ms: now_ms(),
        provider: "system".to_string(),
        focused_app: Some(focused_app),
        focused_window,
        caret,
        pointer: pointer_context(),
    })
}

unsafe fn app_info(app: AXUIElementRef) -> AppInfo {
    let mut pid = 0;

    unsafe { AXUIElementGetPid(app, &mut pid) };

    AppInfo {
        name: unsafe { copy_string(app, TITLE) }.unwrap_or_default(),
        bundle_id: None,
        pid,
    }
}

unsafe fn window_info(window: AXUIElementRef) -> WindowInfo {
    WindowInfo {
        title: unsafe { copy_string(window, TITLE) },
        bounds: unsafe { bounds(window) },
    }
}

unsafe fn caret_context(element: AXUIElementRef) -> CaretContext {
    CaretContext {
        element: Some(unsafe { element_info(element) }),
        text_before: String::new(),
        text_after: String::new(),
        selected_text: None,
        line: None,
        column: None,
        bounds: None,
    }
}

unsafe fn element_info(element: AXUIElementRef) -> ElementInfo {
    let role = unsafe { copy_string(element, ROLE) }.unwrap_or_default();
    let editable = role == "AXTextArea" || role == "AXTextField" || role == "AXComboBox";

    ElementInfo {
        label: unsafe { copy_string(element, TITLE) }
            .or_else(|| unsafe { copy_string(element, DESCRIPTION) }),
        value: None,
        bounds: unsafe { bounds(element) },
        role,
        editable,
    }
}

fn pointer_context() -> Option<PointerContext> {
    let event = unsafe { CGEventCreate(ptr::null()) };

    if event.is_null() {
        return None;
    }

    let location = unsafe { CGEventGetLocation(event) };

    unsafe { CFRelease(event) };

    Some(PointerContext {
        position: Point {
            x: location.x,
            y: location.y,
        },
        element: None,
        app: None,
        window: None,
    })
}

unsafe fn bounds(element: AXUIElementRef) -> Option<Rect> {
    let position = unsafe { copy_attribute(element, POSITION) }?;
    let size = unsafe { copy_attribute(element, SIZE) }?;

    let mut origin = CGPoint::default();
    let mut extent = CGSize::default();

    let read_origin = unsafe {
        AXValueGetValue(
            position,
            AX_VALUE_CG_POINT,
            &mut origin as *mut CGPoint as *mut c_void,
        )
    };
    let read_extent = unsafe {
        AXValueGetValue(
            size,
            AX_VALUE_CG_SIZE,
            &mut extent as *mut CGSize as *mut c_void,
        )
    };

    unsafe { CFRelease(position) };
    unsafe { CFRelease(size) };

    if !read_origin || !read_extent {
        return None;
    }

    Some(Rect {
        x: origin.x,
        y: origin.y,
        width: extent.width,
        height: extent.height,
    })
}

unsafe fn copy_attribute(element: AXUIElementRef, attribute: &str) -> Option<CFTypeRef> {
    let name = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let result =
        unsafe { AXUIElementCopyAttributeValue(element, name.as_concrete_TypeRef(), &mut value) };

    if result != 0 || value.is_null() {
        return None;
    }

    Some(value)
}

unsafe fn copy_element(element: AXUIElementRef, attribute: &str) -> Option<AXUIElementRef> {
    let value = unsafe { copy_attribute(element, attribute) }?;

    Some(value as AXUIElementRef)
}

unsafe fn copy_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let value = unsafe { copy_attribute(element, attribute) }?;
    let text = unsafe { CFString::wrap_under_create_rule(value as CFStringRef) }.to_string();

    if text.is_empty() {
        return None;
    }

    Some(text)
}
