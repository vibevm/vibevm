specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

#[cfg(any(test, feature = "inject-failures"))]
pub type OwnedTreeCheckHook = Box<dyn Fn(&Pinned, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_before_owned_tree_check(hook: Option<OwnedTreeCheckHook>) {
    tree_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod manifest_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned)>;
    thread_local! {
        static BETWEEN: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        BETWEEN.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn between(root: &crate::Pinned) {
        let hook = BETWEEN.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(root);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod manifest_hook {
    pub fn between(_: &crate::Pinned) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type ManifestPassHook = Box<dyn Fn(&Pinned)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_between_manifest_passes(hook: Option<ManifestPassHook>) {
    manifest_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod lease_hook {
    use std::cell::RefCell;
    type Hook = Box<dyn Fn(&crate::Pinned)>;
    thread_local! {
        static DURING: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        DURING.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn during(root: &crate::Pinned) {
        let hook = DURING.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(root);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod lease_hook {
    pub fn during(_: &crate::Pinned) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type LeaseAcquisitionHook = Box<dyn Fn(&Pinned)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_during_manifest_lease(hook: Option<LeaseAcquisitionHook>) {
    lease_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod publish_hook {
    use std::cell::RefCell;
    type Hook = Box<dyn Fn(&crate::Pinned, &str)>;
    thread_local! {
        static BEFORE_MOVE: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static AFTER_MOVE: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm_before(hook: Option<Hook>) {
        BEFORE_MOVE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn arm_after(hook: Option<Hook>) {
        AFTER_MOVE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn before_move(parent: &crate::Pinned, name: &str) {
        let hook = BEFORE_MOVE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
    pub fn after_move(parent: &crate::Pinned, name: &str) {
        let hook = AFTER_MOVE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod publish_hook {
    pub fn before_move(_: &crate::Pinned, _: &str) {}
    pub fn after_move(_: &crate::Pinned, _: &str) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type OwnedPublishHook = Box<dyn Fn(&Pinned, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_before_owned_tree_publish(hook: Option<OwnedPublishHook>) {
    publish_hook::arm_before(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_after_owned_tree_publish_move(hook: Option<OwnedPublishHook>) {
    publish_hook::arm_after(hook);
}
