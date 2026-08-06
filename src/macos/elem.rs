use std::ffi::c_void;
use std::ptr;

use accessibility_sys::{
    AXUIElementCopyAttributeNames, AXUIElementCopyAttributeValue,
    AXUIElementCopyParameterizedAttributeNames, AXUIElementCopyParameterizedAttributeValue,
    AXUIElementRef, AXUIElementSetAttributeValue, AXUIElementSetMessagingTimeout, AXValueCreate,
    AXValueGetValue, AXValueRef, kAXBoundsForRangeParameterizedAttribute, kAXDescriptionAttribute,
    kAXEnabledAttribute, kAXFocusedUIElementAttribute, kAXHelpAttribute,
    kAXNumberOfCharactersAttribute, kAXPlaceholderValueAttribute, kAXRoleAttribute,
    kAXRoleDescriptionAttribute, kAXSelectedTextRangeAttribute, kAXSubroleAttribute,
    kAXTitleAttribute, kAXValueAttribute, kAXValueTypeCFRange, kAXValueTypeCGPoint,
    kAXValueTypeCGRect, kAXValueTypeCGSize,
};
use core_foundation::array::CFArray;
use core_foundation::base::{CFRange, CFRelease, CFRetain, CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};

use crate::model::{ElementInfo, Rect};

/// Reads of a focused element happen on every keystroke and must never stall the caller, so they get
/// a tight timeout. Turning on a Chromium accessibility tree is slow enough to blow through it, so
/// the application element gets a generous one instead.
pub const ELEMENT_TIMEOUT: f32 = 0.25;
pub const APP_TIMEOUT: f32 = 2.0;

pub const TEXT_ROLES: [&str; 5] = [
    "AXTextField",
    "AXTextArea",
    "AXComboBox",
    "AXSearchField",
    "AXWebArea",
];

pub const MAX_CARET_HEIGHT: f64 = 120.0;
const FRAME_TOLERANCE: f64 = 24.0;

pub struct Elem(pub AXUIElementRef);

impl Elem {
    pub fn from_create_rule(raw: AXUIElementRef, timeout: f32) -> Option<Self> {
        if raw.is_null() {
            return None;
        }

        unsafe { AXUIElementSetMessagingTimeout(raw, timeout) };

        Some(Elem(raw))
    }

    pub fn from_get_rule(raw: AXUIElementRef, timeout: f32) -> Option<Self> {
        if raw.is_null() {
            return None;
        }

        unsafe { CFRetain(raw as CFTypeRef) };

        Self::from_create_rule(raw, timeout)
    }

    pub fn set_flag(&self, name: &str) -> i32 {
        unsafe {
            AXUIElementSetAttributeValue(
                self.0,
                CFString::new(name).as_concrete_TypeRef(),
                CFBoolean::true_value().as_CFTypeRef(),
            )
        }
    }

    /// Chromium builds its accessibility tree lazily and exposes nothing until it detects an
    /// assistive client, which is why Electron and Chrome look empty to a naive reader. Electron
    /// takes `AXManualAccessibility`; Chrome rejects that one and only answers to
    /// `AXEnhancedUserInterface`, the flag VoiceOver sets. Whichever the app does not understand is
    /// a harmless no-op.
    pub fn enable_accessibility_tree(&self) {
        self.set_flag("AXManualAccessibility");
        self.set_flag("AXEnhancedUserInterface");
    }

    /// An app's focused element is often a window or web area that in turn points at the element
    /// really holding the caret, so follow the chain down to the deepest one.
    pub fn descend_to_focused(self) -> Elem {
        let mut current = self;

        for _ in 0..8 {
            match current.element_attribute(kAXFocusedUIElementAttribute) {
                Some(next) => current = next,
                None => return current,
            }
        }

        current
    }

    pub fn attribute(&self, name: &str) -> Option<CFType> {
        let key = CFString::new(name);
        let mut out: CFTypeRef = ptr::null();
        let err =
            unsafe { AXUIElementCopyAttributeValue(self.0, key.as_concrete_TypeRef(), &mut out) };

        if err != 0 || out.is_null() {
            return None;
        }

        Some(unsafe { CFType::wrap_under_create_rule(out) })
    }

    pub fn string_attribute(&self, name: &str) -> Option<String> {
        let text = self.attribute(name)?.downcast::<CFString>()?.to_string();

        if text.is_empty() {
            return None;
        }

        Some(text)
    }

    pub fn number_attribute(&self, name: &str) -> Option<i64> {
        self.attribute(name)?.downcast::<CFNumber>()?.to_i64()
    }

    pub fn bool_attribute(&self, name: &str) -> Option<bool> {
        Some(self.attribute(name)?.downcast::<CFBoolean>()?.into())
    }

    pub fn element_attribute(&self, name: &str) -> Option<Elem> {
        let value = self.attribute(name)?;
        let raw = value.as_CFTypeRef() as AXUIElementRef;

        if raw.is_null() {
            return None;
        }

        std::mem::forget(value);

        Elem::from_create_rule(raw, ELEMENT_TIMEOUT)
    }

    pub fn children(&self) -> Vec<Elem> {
        self.children_of("AXChildren")
    }

    pub fn children_of(&self, name: &str) -> Vec<Elem> {
        let Some(value) = self.attribute(name) else {
            return Vec::new();
        };
        let array = value.as_CFTypeRef() as CFArrayRef;

        if array.is_null() {
            return Vec::new();
        }

        let count = unsafe { CFArrayGetCount(array) };
        let mut children = Vec::new();

        for index in 0..count {
            let raw = unsafe { CFArrayGetValueAtIndex(array, index) } as AXUIElementRef;

            if let Some(child) = Elem::from_get_rule(raw, ELEMENT_TIMEOUT) {
                children.push(child);
            }
        }

        children
    }

    pub fn attribute_names(&self) -> Vec<String> {
        let mut names: CFArrayRef = ptr::null();
        let err = unsafe { AXUIElementCopyAttributeNames(self.0, &mut names) };

        if err != 0 || names.is_null() {
            return Vec::new();
        }

        let names: CFArray<CFString> = unsafe { CFArray::wrap_under_create_rule(names) };

        names.iter().map(|name| name.to_string()).collect()
    }

    pub fn parameterized_attribute_names(&self) -> Vec<String> {
        let mut names: CFArrayRef = ptr::null();
        let err = unsafe { AXUIElementCopyParameterizedAttributeNames(self.0, &mut names) };

        if err != 0 || names.is_null() {
            return Vec::new();
        }

        let names: CFArray<CFString> = unsafe { CFArray::wrap_under_create_rule(names) };

        names.iter().map(|name| name.to_string()).collect()
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        let value = self.attribute(kAXSelectedTextRangeAttribute)?;
        let mut range = CFRange {
            location: 0,
            length: 0,
        };
        let ok = unsafe {
            AXValueGetValue(
                value.as_CFTypeRef() as AXValueRef,
                kAXValueTypeCFRange,
                &mut range as *mut _ as *mut c_void,
            )
        };

        if !ok || range.location < 0 {
            return None;
        }

        Some((range.location as usize, range.length.max(0) as usize))
    }

    pub fn bounds_for_range(&self, location: usize, length: usize) -> Option<Rect> {
        let range = CFRange {
            location: location as isize,
            length: length as isize,
        };
        let param =
            unsafe { AXValueCreate(kAXValueTypeCFRange, &range as *const _ as *const c_void) };

        if param.is_null() {
            return None;
        }

        let param = unsafe { CFType::wrap_under_create_rule(param as CFTypeRef) };
        let key = CFString::new(kAXBoundsForRangeParameterizedAttribute);
        let mut out: CFTypeRef = ptr::null();
        let err = unsafe {
            AXUIElementCopyParameterizedAttributeValue(
                self.0,
                key.as_concrete_TypeRef(),
                param.as_CFTypeRef(),
                &mut out,
            )
        };

        if err != 0 || out.is_null() {
            return None;
        }

        let out = unsafe { CFType::wrap_under_create_rule(out) };
        let mut rect = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0));
        let ok = unsafe {
            AXValueGetValue(
                out.as_CFTypeRef() as AXValueRef,
                kAXValueTypeCGRect,
                &mut rect as *mut _ as *mut c_void,
            )
        };

        if !ok {
            return None;
        }

        Some(Rect {
            x: rect.origin.x,
            y: rect.origin.y,
            width: rect.size.width,
            height: rect.size.height,
        })
    }

    pub fn frame(&self) -> Option<Rect> {
        let position = self.attribute(accessibility_sys::kAXPositionAttribute)?;
        let size = self.attribute(accessibility_sys::kAXSizeAttribute)?;

        let mut point = CGPoint::new(0.0, 0.0);
        let mut dims = CGSize::new(0.0, 0.0);
        let got_point = unsafe {
            AXValueGetValue(
                position.as_CFTypeRef() as AXValueRef,
                kAXValueTypeCGPoint,
                &mut point as *mut _ as *mut c_void,
            )
        };
        let got_size = unsafe {
            AXValueGetValue(
                size.as_CFTypeRef() as AXValueRef,
                kAXValueTypeCGSize,
                &mut dims as *mut _ as *mut c_void,
            )
        };

        if !got_point || !got_size {
            return None;
        }

        Some(Rect {
            x: point.x,
            y: point.y,
            width: dims.width,
            height: dims.height,
        })
    }

    pub fn caret_rect(&self, selection_start: usize) -> Option<Rect> {
        let frame = self.frame();

        if let Some(rect) = self.bounds_for_range(selection_start, 0)
            && is_caret_like(&rect, frame.as_ref())
        {
            return Some(rect);
        }

        let total = self
            .number_attribute(kAXNumberOfCharactersAttribute)
            .unwrap_or(0) as usize;

        if selection_start < total
            && let Some(rect) = self.bounds_for_range(selection_start, 1)
            && is_caret_like(&rect, frame.as_ref())
        {
            return Some(Rect { width: 0.0, ..rect });
        }

        if selection_start > 0
            && let Some(rect) = self.bounds_for_range(selection_start - 1, 1)
            && is_caret_like(&rect, frame.as_ref())
        {
            return Some(Rect {
                x: rect.x + rect.width,
                width: 0.0,
                ..rect
            });
        }

        let frame = frame?;

        if frame.width <= 0.0 || frame.height <= 0.0 {
            return None;
        }

        Some(Rect {
            x: frame.x,
            y: frame.y,
            width: 0.0,
            height: frame.height.min(MAX_CARET_HEIGHT),
        })
    }

    pub fn info(&self, attribute_names: bool) -> ElementInfo {
        let role = self.string_attribute(kAXRoleAttribute).unwrap_or_default();
        let subrole = self.string_attribute(kAXSubroleAttribute);
        let editable = TEXT_ROLES.contains(&role.as_str())
            || subrole.as_deref() == Some("AXContentEditable")
            || self.selection().is_some();

        ElementInfo {
            label: self
                .string_attribute(kAXTitleAttribute)
                .or_else(|| self.string_attribute(kAXDescriptionAttribute)),
            role_description: self.string_attribute(kAXRoleDescriptionAttribute),
            help: self.string_attribute(kAXHelpAttribute),
            placeholder: self.string_attribute(kAXPlaceholderValueAttribute),
            identifier: self.string_attribute("AXIdentifier"),
            value: self.string_attribute(kAXValueAttribute),
            bounds: self.frame(),
            enabled: self.bool_attribute(kAXEnabledAttribute).unwrap_or(true),
            character_count: self
                .number_attribute(kAXNumberOfCharactersAttribute)
                .map(|count| count.max(0) as usize),
            attributes: if attribute_names {
                self.attribute_names()
            } else {
                Vec::new()
            },
            parameterized_attributes: if attribute_names {
                self.parameterized_attribute_names()
            } else {
                Vec::new()
            },
            role,
            subrole,
            editable,
        }
    }
}

impl Drop for Elem {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0 as CFTypeRef) };
    }
}

/// A rectangle answered for a zero-length range is only the caret when it is plausibly a thin,
/// text-sized bar inside the element that reported it; apps that do not really support the query
/// answer with the whole window, an empty rect at the origin, or nonsense.
fn is_caret_like(rect: &Rect, frame: Option<&Rect>) -> bool {
    if !(rect.x.is_finite() && rect.y.is_finite() && rect.height.is_finite()) {
        return false;
    }
    if rect.height <= 0.0 || rect.height > MAX_CARET_HEIGHT {
        return false;
    }
    if rect.x == 0.0 && rect.y == 0.0 {
        return false;
    }

    let Some(frame) = frame else {
        return true;
    };

    if frame.width <= 0.0 || frame.height <= 0.0 {
        return true;
    }

    rect.x >= frame.x - FRAME_TOLERANCE
        && rect.x <= frame.x + frame.width + FRAME_TOLERANCE
        && rect.y >= frame.y - FRAME_TOLERANCE
        && rect.y <= frame.y + frame.height + FRAME_TOLERANCE
}
