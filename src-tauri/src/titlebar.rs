use std::ffi::{CStr, CString, c_void};
use std::ptr;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
use objc2::{msg_send, sel};
use objc2_app_kit::{NSView, NSWindow, NSWindowButton};
use objc2_foundation::NSPoint;

struct Subclass {
    superclass: &'static AnyClass,
    class: &'static AnyClass,
}

static SUBCLASSES: Mutex<Vec<Subclass>> = Mutex::new(Vec::new());

pub fn install(ns_window: *mut c_void) {
    if ns_window.is_null() {
        return;
    }
    let Some(window) = (unsafe { Retained::retain(ns_window.cast::<NSWindow>()) }) else {
        return;
    };
    let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
        return;
    };
    let Some(buttons) = (unsafe { close.superview() }) else {
        return;
    };
    let Some(container) = (unsafe { buttons.superview() }) else {
        return;
    };
    let class = container_class(&container);
    if superclass_of(class).is_some() {
        return;
    }
    let passthrough = passthrough_for(class);
    let object = unsafe { &*ptr::from_ref(&*container).cast::<AnyObject>() };
    let current: *const AnyClass = unsafe { msg_send![object, class] };
    if !ptr::eq(current, ptr::from_ref(class)) {
        return;
    }
    let _: &AnyClass = unsafe { AnyObject::set_class(object, passthrough) };
}

fn container_class(container: &NSView) -> &'static AnyClass {
    let class: *const AnyClass = unsafe { msg_send![container, class] };
    unsafe { &*class }
}

fn passthrough_for(superclass: &'static AnyClass) -> &'static AnyClass {
    let mut classes = SUBCLASSES.lock().expect("titlebar classes");
    if let Some(found) = classes
        .iter()
        .find(|item| ptr::eq(item.superclass, superclass))
    {
        return found.class;
    }
    let name =
        CString::new(format!("SqlgTitlebarPassthrough{}", classes.len())).expect("class name");
    let name: &'static CStr = Box::leak(name.into_boxed_c_str());
    let class = register_passthrough(name, superclass);
    classes.push(Subclass { superclass, class });
    class
}

fn superclass_of(class: &AnyClass) -> Option<&'static AnyClass> {
    SUBCLASSES
        .lock()
        .expect("titlebar classes")
        .iter()
        .find(|item| ptr::eq(item.class, class))
        .map(|item| item.superclass)
}

fn register_passthrough(name: &'static CStr, superclass: &AnyClass) -> &'static AnyClass {
    if let Some(existing) = AnyClass::get(name) {
        return existing;
    }
    let mut builder = ClassBuilder::new(name, superclass).expect("titlebar class");
    unsafe {
        builder.add_method(
            sel!(hitTest:),
            hit_test as unsafe extern "C" fn(_, _, _) -> _,
        );
    }
    builder.register()
}

unsafe extern "C" fn hit_test(this: &NSView, _: Sel, point: NSPoint) -> *mut AnyObject {
    let class_ptr: *const AnyClass = unsafe { msg_send![this, class] };
    let Some(superclass) = superclass_of(unsafe { &*class_ptr }) else {
        return ptr::null_mut();
    };
    let hit: *mut AnyObject = unsafe { msg_send![super(this, superclass), hitTest: point] };
    if hit.is_null() || !unsafe { is_traffic_light(this, hit) } {
        ptr::null_mut()
    } else {
        hit
    }
}

unsafe fn is_traffic_light(container: &NSView, mut view: *mut AnyObject) -> bool {
    let Some(window) = container.window() else {
        return false;
    };
    let buttons = [
        window.standardWindowButton(NSWindowButton::CloseButton),
        window.standardWindowButton(NSWindowButton::MiniaturizeButton),
        window.standardWindowButton(NSWindowButton::ZoomButton),
    ];
    let targets: Vec<*mut AnyObject> = buttons
        .into_iter()
        .flatten()
        .map(|button| Retained::as_ptr(&button).cast::<AnyObject>().cast_mut())
        .collect();
    let container_ptr = ptr::from_ref(container).cast::<AnyObject>();
    for _ in 0..8 {
        if view.is_null() || ptr::eq(view, container_ptr) {
            return false;
        }
        if targets.iter().any(|target| ptr::eq(view, *target)) {
            return true;
        }
        view = unsafe { msg_send![view, superview] };
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2::ClassType;

    #[test]
    fn each_superclass_gets_its_own_subclass() {
        let first = NSView::class();
        let name = c"SqlgTitlebarFixture";
        let second = AnyClass::get(name).unwrap_or_else(|| {
            ClassBuilder::new(name, NSView::class())
                .expect("fixture")
                .register()
        });
        let subclass_a = passthrough_for(first);
        let subclass_b = passthrough_for(second);
        assert!(!ptr::eq(subclass_a, subclass_b));
        assert!(ptr::eq(passthrough_for(first), subclass_a));
        assert!(
            subclass_a
                .superclass()
                .is_some_and(|class| ptr::eq(class, first))
        );
        assert!(
            subclass_b
                .superclass()
                .is_some_and(|class| ptr::eq(class, second))
        );
        assert!(superclass_of(subclass_a).is_some_and(|class| ptr::eq(class, first)));
        assert!(superclass_of(subclass_b).is_some_and(|class| ptr::eq(class, second)));
    }
}
