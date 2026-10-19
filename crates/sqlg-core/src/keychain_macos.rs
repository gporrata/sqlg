//! Login-keychain passwords for sqlg connections.
//!
//! The app is ad-hoc signed, so an ACL that names the creating binary asks for
//! the login keychain password after every rebuild (`kSecKeychainPromptUnsigned`
//! keeps "Always Allow" from sticking). Items are stored the way
//! `security add-generic-password -A` stores them: every ACL trusts any
//! application and has prompt selector 0. Any process on this Mac can read
//! these MySQL passwords. Unsigned rebuilds stay silent.
//!
//! A process-wide cache covers the app creating a fresh store per command.
//! `loadSchema` and `ping` otherwise decrypt the same item at the same time.

use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::{LazyLock, Mutex, MutexGuard};

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{Boolean, CFRelease, CFTypeRef, OSStatus};
use core_foundation_sys::string::CFStringRef;
use security_framework::base::Error;
#[cfg(test)]
use security_framework::os::macos::keychain::{CreateOptions, KeychainSettings};
use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
use security_framework_sys::base::{
    SecAccessRef, SecKeychainAttribute, SecKeychainAttributeList, SecKeychainItemRef,
    SecKeychainRef, errSecAuthFailed, errSecDuplicateItem, errSecItemNotFound, errSecParam,
    errSecSuccess,
};
use security_framework_sys::keychain::{
    SecKeychainAddGenericPassword, SecKeychainFindGenericPassword,
    SecKeychainGetUserInteractionAllowed, SecKeychainSetUserInteractionAllowed,
};
use security_framework_sys::keychain_item::{SecKeychainItemDelete, SecKeychainItemFreeContent};

use crate::BUNDLE_ID;
use crate::store::StoreError;

const ACCOUNT_TAG: u32 = u32::from_be_bytes(*b"acct");
const SERVICE_TAG: u32 = u32::from_be_bytes(*b"svce");
const LABEL_TAG: u32 = u32::from_be_bytes(*b"labl");
const GENERIC_PASSWORD: u32 = u32::from_be_bytes(*b"genp");
const ERR_USER_CANCELED: OSStatus = -128;
const ERR_INTERACTION_NOT_ALLOWED: OSStatus = -25308;
const LABEL: &[u8] = b"sqlg";

type SecACLRef = *mut c_void;
type SecKeychainPromptSelector = u16;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecAccessCreate(
        descriptor: CFStringRef,
        trusted_list: CFArrayRef,
        access: *mut SecAccessRef,
    ) -> OSStatus;

    fn SecAccessCopyACLList(access: SecAccessRef, acl_list: *mut CFArrayRef) -> OSStatus;

    fn SecACLCopyContents(
        acl: SecACLRef,
        application_list: *mut CFArrayRef,
        description: *mut CFStringRef,
        prompt_selector: *mut SecKeychainPromptSelector,
    ) -> OSStatus;

    fn SecACLSetContents(
        acl: SecACLRef,
        application_list: CFArrayRef,
        description: CFStringRef,
        prompt_selector: SecKeychainPromptSelector,
    ) -> OSStatus;

    fn SecKeychainItemCreateFromContent(
        item_class: u32,
        attr_list: *mut SecKeychainAttributeList,
        length: u32,
        data: *const c_void,
        keychain: SecKeychainRef,
        initial_access: SecAccessRef,
        item: *mut SecKeychainItemRef,
    ) -> OSStatus;

    fn SecKeychainItemCopyAccess(item: SecKeychainItemRef, access: *mut SecAccessRef) -> OSStatus;
}

static GATE: Mutex<()> = Mutex::new(());
static CACHE: LazyLock<Mutex<HashMap<CacheKey, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(PartialEq, Eq, Hash)]
struct CacheKey {
    scope: String,
    account: String,
}

/// Login-keychain handle used by [`crate::Secrets`]. Fields stay private.
pub struct Keychain {
    keychain: SecKeychain,
    scope: String,
}

impl Keychain {
    pub(crate) fn user_default() -> Result<Self, StoreError> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(framework_error)?;
        Ok(Self {
            keychain,
            scope: "user-domain".to_string(),
        })
    }

    pub(crate) fn get(&self, account: &str) -> Result<Option<String>, StoreError> {
        let _gate = lock_gate();
        if let Some(secret) = self.cached(account) {
            return Ok(Some(secret));
        }
        let secret = self.read_uncached(account)?;
        if let Some(secret) = &secret {
            self.remember(account, secret);
        }
        Ok(secret)
    }

    pub(crate) fn set(&self, account: &str, secret: &str) -> Result<(), StoreError> {
        let _gate = lock_gate();
        let _ui = InteractionOff::enter()?;
        self.forget(account);
        match create_open(&self.keychain, account, secret) {
            Ok(()) => {}
            Err(status) if status == errSecDuplicateItem => {
                delete_item(&self.keychain, account).map_err(status_error)?;
                if let Err(status) = create_open(&self.keychain, account, secret) {
                    // The old item is gone. Put the new secret back even if the
                    // open ACL could not be written, so the password is not lost.
                    let _ = add_default(&self.keychain, account, secret);
                    return Err(status_error(status));
                }
            }
            Err(status) => return Err(status_error(status)),
        }
        self.remember(account, secret);
        Ok(())
    }

    pub(crate) fn delete(&self, account: &str) -> Result<(), StoreError> {
        let _gate = lock_gate();
        let _ui = InteractionOff::enter()?;
        delete_item(&self.keychain, account).map_err(status_error)?;
        self.forget(account);
        Ok(())
    }

    fn read_uncached(&self, account: &str) -> Result<Option<String>, StoreError> {
        // Drop the interaction guard before any prompting find. The gate stays
        // held so a second caller waits and then hits the cache.
        let preflight = {
            let _ui = InteractionOff::enter()?;
            find_secret(&self.keychain, account)
        };
        let found = match preflight {
            Ok(found) => found,
            Err(error) if error.needs_prompt() => {
                find_secret(&self.keychain, account).map_err(FindError::into_store)?
            }
            Err(error) => return Err(error.into_store()),
        };
        let Some(found) = found else {
            return Ok(None);
        };
        {
            let _ui = InteractionOff::enter()?;
            // A failed rewrite must not fail the read: the secret is already
            // in memory. User cancel never reaches this point.
            repair_acl(&self.keychain, account, &found);
        }
        Ok(Some(found.secret))
    }

    fn cache_key(&self, account: &str) -> CacheKey {
        CacheKey {
            scope: self.scope.clone(),
            account: account.to_string(),
        }
    }

    fn cached(&self, account: &str) -> Option<String> {
        cache().get(&self.cache_key(account)).cloned()
    }

    fn remember(&self, account: &str, secret: &str) {
        cache().insert(self.cache_key(account), secret.to_string());
    }

    fn forget(&self, account: &str) {
        cache().remove(&self.cache_key(account));
    }
}

#[cfg(test)]
impl Keychain {
    fn create(path: &std::path::Path, password: &str) -> Result<Self, StoreError> {
        let mut keychain = CreateOptions::new()
            .password(password)
            .create(path)
            .map_err(framework_error)?;
        keychain.unlock(Some(password)).map_err(framework_error)?;
        keychain
            .set_settings(&KeychainSettings::new())
            .map_err(framework_error)?;
        Ok(Self {
            keychain,
            scope: path.to_string_lossy().into_owned(),
        })
    }

    fn access_is_open(&self, account: &str) -> Result<bool, StoreError> {
        let _gate = lock_gate();
        let _ui = InteractionOff::enter()?;
        let item = find_item(&self.keychain, account).map_err(status_error)?;
        let Some(item) = item else {
            return Err(StoreError::Keychain("item not found".into()));
        };
        Ok(acl_is_open(item.0))
    }

    fn forget_cached(&self, account: &str) {
        let _gate = lock_gate();
        self.forget(account);
    }

    fn describe_access(&self, account: &str) -> String {
        let _gate = lock_gate();
        let _ui = InteractionOff::enter().ok();
        let Ok(Some(item)) = find_item(&self.keychain, account) else {
            return "item missing".to_string();
        };
        describe_item(item.0)
    }
}

struct Item(SecKeychainItemRef);

impl Drop for Item {
    fn drop(&mut self) {
        release_item(self.0);
        self.0 = std::ptr::null_mut();
    }
}

struct Found {
    secret: String,
    item: Item,
}

struct Access(SecAccessRef);

impl Drop for Access {
    fn drop(&mut self) {
        release(self.0 as CFTypeRef);
        self.0 = std::ptr::null_mut();
    }
}

struct Cf(CFTypeRef);

impl Drop for Cf {
    fn drop(&mut self) {
        release(self.0);
        self.0 = std::ptr::null();
    }
}

#[must_use = "restores keychain prompts when dropped"]
struct InteractionOff {
    previous: Boolean,
}

impl InteractionOff {
    fn enter() -> Result<Self, StoreError> {
        unsafe {
            let mut previous: Boolean = 1;
            check(SecKeychainGetUserInteractionAllowed(&mut previous))?;
            check(SecKeychainSetUserInteractionAllowed(0))?;
            Ok(Self { previous })
        }
    }
}

impl Drop for InteractionOff {
    fn drop(&mut self) {
        unsafe {
            SecKeychainSetUserInteractionAllowed(self.previous);
        }
    }
}

enum FindError {
    Status(OSStatus),
    NotUtf8,
}

impl FindError {
    fn needs_prompt(&self) -> bool {
        matches!(
            self,
            Self::Status(status)
                if *status == errSecAuthFailed || *status == ERR_INTERACTION_NOT_ALLOWED
        )
    }

    fn into_store(self) -> StoreError {
        match self {
            Self::NotUtf8 => StoreError::Keychain("stored password is not valid UTF-8".into()),
            Self::Status(ERR_USER_CANCELED) => {
                StoreError::Keychain("keychain access was canceled".into())
            }
            Self::Status(status) => status_error(status),
        }
    }
}

fn lock_gate() -> MutexGuard<'static, ()> {
    GATE.lock().unwrap_or_else(|err| err.into_inner())
}

fn cache() -> MutexGuard<'static, HashMap<CacheKey, String>> {
    CACHE.lock().unwrap_or_else(|err| err.into_inner())
}

fn framework_error(err: Error) -> StoreError {
    StoreError::Keychain(err.to_string())
}

fn status_error(status: OSStatus) -> StoreError {
    let detail = Error::from_code(status);
    StoreError::Keychain(format!("{detail} ({status})"))
}

fn check(status: OSStatus) -> Result<(), StoreError> {
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(status_error(status))
    }
}

fn release(reference: CFTypeRef) {
    if !reference.is_null() {
        unsafe { CFRelease(reference) }
    }
}

fn release_item(item: SecKeychainItemRef) {
    release(item as CFTypeRef);
}

fn free_password(data: *mut c_void) {
    if !data.is_null() {
        unsafe {
            SecKeychainItemFreeContent(std::ptr::null_mut(), data);
        }
    }
}

fn secret_bytes(secret: &str) -> (*const c_void, u32) {
    if secret.is_empty() {
        (std::ptr::null(), 0)
    } else {
        (secret.as_ptr().cast(), secret.len() as u32)
    }
}

/// `SecAccessCreate` with a null trusted list trusts only this binary.
/// Widen every ACL before the item is saved. A null application list means
/// any application, and prompt selector 0 does not ask.
fn open_access() -> Result<Access, OSStatus> {
    let descriptor = CFString::new("sqlg");
    let mut created: SecAccessRef = std::ptr::null_mut();
    let status = unsafe {
        SecAccessCreate(
            descriptor.as_concrete_TypeRef(),
            std::ptr::null(),
            &mut created,
        )
    };
    if status != errSecSuccess || created.is_null() {
        return Err(if status == errSecSuccess {
            errSecParam
        } else {
            status
        });
    }
    let access = Access(created);
    let mut list: CFArrayRef = std::ptr::null();
    let status = unsafe { SecAccessCopyACLList(access.0, &mut list) };
    if status != errSecSuccess || list.is_null() {
        return Err(if status == errSecSuccess {
            errSecParam
        } else {
            status
        });
    }
    let _list = Cf(list as CFTypeRef);
    let count = unsafe { CFArrayGetCount(list) };
    if count == 0 {
        return Err(errSecParam);
    }
    let description = CFString::new("sqlg");
    for index in 0..count {
        let acl = unsafe { CFArrayGetValueAtIndex(list, index) } as SecACLRef;
        if acl.is_null() {
            return Err(errSecParam);
        }
        let status = unsafe {
            SecACLSetContents(acl, std::ptr::null(), description.as_concrete_TypeRef(), 0)
        };
        if status != errSecSuccess {
            return Err(status);
        }
    }
    Ok(access)
}

fn attribute(tag: u32, bytes: &[u8]) -> SecKeychainAttribute {
    SecKeychainAttribute {
        tag,
        length: bytes.len() as u32,
        // Security copies the bytes before returning and does not write them.
        data: bytes.as_ptr() as *mut c_void,
    }
}

fn create_open(keychain: &SecKeychain, account: &str, secret: &str) -> Result<(), OSStatus> {
    let access = open_access()?;
    let service = BUNDLE_ID.as_bytes();
    let mut attributes = [
        attribute(ACCOUNT_TAG, account.as_bytes()),
        attribute(SERVICE_TAG, service),
        attribute(LABEL_TAG, LABEL),
    ];
    let mut list = SecKeychainAttributeList {
        count: attributes.len() as u32,
        attr: attributes.as_mut_ptr(),
    };
    let (data, length) = secret_bytes(secret);
    let status = unsafe {
        SecKeychainItemCreateFromContent(
            GENERIC_PASSWORD,
            &mut list,
            length,
            data,
            keychain.as_concrete_TypeRef(),
            access.0,
            std::ptr::null_mut(),
        )
    };
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(status)
    }
}

fn add_default(keychain: &SecKeychain, account: &str, secret: &str) -> Result<(), OSStatus> {
    let service = BUNDLE_ID;
    let (data, length) = secret_bytes(secret);
    let status = unsafe {
        SecKeychainAddGenericPassword(
            keychain.as_concrete_TypeRef(),
            service.len() as u32,
            service.as_ptr().cast(),
            account.len() as u32,
            account.as_ptr().cast(),
            length,
            data,
            std::ptr::null_mut(),
        )
    };
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(status)
    }
}

fn find_secret(keychain: &SecKeychain, account: &str) -> Result<Option<Found>, FindError> {
    let service = BUNDLE_ID;
    let mut length = 0u32;
    let mut data: *mut c_void = std::ptr::null_mut();
    let mut item: SecKeychainItemRef = std::ptr::null_mut();
    let status = unsafe {
        SecKeychainFindGenericPassword(
            keychain.as_concrete_TypeRef() as CFTypeRef,
            service.len() as u32,
            service.as_ptr().cast(),
            account.len() as u32,
            account.as_ptr().cast(),
            &mut length,
            &mut data,
            &mut item,
        )
    };
    if status == errSecItemNotFound {
        free_password(data);
        release_item(item);
        return Ok(None);
    }
    if status != errSecSuccess {
        free_password(data);
        release_item(item);
        return Err(FindError::Status(status));
    }
    if length > 0 && data.is_null() {
        release_item(item);
        return Err(FindError::Status(errSecParam));
    }
    let secret = if length == 0 {
        String::new()
    } else {
        let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
        match String::from_utf8(bytes.to_vec()) {
            Ok(secret) => secret,
            Err(_) => {
                free_password(data);
                release_item(item);
                return Err(FindError::NotUtf8);
            }
        }
    };
    free_password(data);
    if item.is_null() {
        return Err(FindError::Status(errSecParam));
    }
    Ok(Some(Found {
        secret,
        item: Item(item),
    }))
}

fn find_item(keychain: &SecKeychain, account: &str) -> Result<Option<Item>, OSStatus> {
    let service = BUNDLE_ID;
    let mut item: SecKeychainItemRef = std::ptr::null_mut();
    let status = unsafe {
        SecKeychainFindGenericPassword(
            keychain.as_concrete_TypeRef() as CFTypeRef,
            service.len() as u32,
            service.as_ptr().cast(),
            account.len() as u32,
            account.as_ptr().cast(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut item,
        )
    };
    if status == errSecItemNotFound {
        return Ok(None);
    }
    if status != errSecSuccess {
        release_item(item);
        return Err(status);
    }
    if item.is_null() {
        return Err(errSecParam);
    }
    Ok(Some(Item(item)))
}

fn delete_item(keychain: &SecKeychain, account: &str) -> Result<(), OSStatus> {
    let Some(item) = find_item(keychain, account)? else {
        return Ok(());
    };
    let status = unsafe { SecKeychainItemDelete(item.0) };
    if status == errSecSuccess || status == errSecItemNotFound {
        Ok(())
    } else {
        Err(status)
    }
}

fn acl_is_open(item: SecKeychainItemRef) -> bool {
    let mut access: SecAccessRef = std::ptr::null_mut();
    let status = unsafe { SecKeychainItemCopyAccess(item, &mut access) };
    if status != errSecSuccess || access.is_null() {
        return false;
    }
    let access = Access(access);
    let mut list: CFArrayRef = std::ptr::null();
    let status = unsafe { SecAccessCopyACLList(access.0, &mut list) };
    if status != errSecSuccess || list.is_null() {
        return false;
    }
    let _list = Cf(list as CFTypeRef);
    let count = unsafe { CFArrayGetCount(list) };
    if count == 0 {
        return false;
    }
    for index in 0..count {
        let acl = unsafe { CFArrayGetValueAtIndex(list, index) } as SecACLRef;
        if acl.is_null() || !acl_allows_any_app(acl) {
            return false;
        }
    }
    true
}

fn acl_allows_any_app(acl: SecACLRef) -> bool {
    let mut apps: CFArrayRef = std::ptr::null();
    let mut description: CFStringRef = std::ptr::null();
    let mut prompt: SecKeychainPromptSelector = u16::MAX;
    let status = unsafe { SecACLCopyContents(acl, &mut apps, &mut description, &mut prompt) };
    if status != errSecSuccess {
        return false;
    }
    let any_app = apps.is_null();
    release(apps as CFTypeRef);
    release(description as CFTypeRef);
    any_app && prompt == 0
}

/// Delete does not prompt, including for an item this binary is not trusted
/// to decrypt. Recreate it with an any-application ACL. If that fails, put the
/// secret back with the default ACL so it is not deleted.
fn repair_acl(keychain: &SecKeychain, account: &str, found: &Found) {
    if acl_is_open(found.item.0) {
        return;
    }
    let status = unsafe { SecKeychainItemDelete(found.item.0) };
    if status != errSecSuccess && status != errSecItemNotFound {
        return;
    }
    if create_open(keychain, account, &found.secret).is_err() {
        let _ = add_default(keychain, account, &found.secret);
    }
}

#[cfg(test)]
fn describe_item(item: SecKeychainItemRef) -> String {
    let mut access: SecAccessRef = std::ptr::null_mut();
    let status = unsafe { SecKeychainItemCopyAccess(item, &mut access) };
    if status != errSecSuccess || access.is_null() {
        return format!("copy access {status}");
    }
    let access = Access(access);
    let mut list: CFArrayRef = std::ptr::null();
    let status = unsafe { SecAccessCopyACLList(access.0, &mut list) };
    if status != errSecSuccess || list.is_null() {
        return format!("copy acl list {status}");
    }
    let _list = Cf(list as CFTypeRef);
    let count = unsafe { CFArrayGetCount(list) };
    let mut parts = vec![format!("{count} acls")];
    for index in 0..count {
        let acl = unsafe { CFArrayGetValueAtIndex(list, index) } as SecACLRef;
        if acl.is_null() {
            parts.push(format!("{index}: null"));
            continue;
        }
        let mut apps: CFArrayRef = std::ptr::null();
        let mut description: CFStringRef = std::ptr::null();
        let mut prompt: SecKeychainPromptSelector = u16::MAX;
        let status = unsafe { SecACLCopyContents(acl, &mut apps, &mut description, &mut prompt) };
        let any_app = apps.is_null();
        let app_count = if apps.is_null() {
            0
        } else {
            unsafe { CFArrayGetCount(apps) }
        };
        release(apps as CFTypeRef);
        release(description as CFTypeRef);
        parts.push(format!(
            "{index}: status {status} any_app {any_app} apps {app_count} prompt {prompt}"
        ));
    }
    parts.join("; ")
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct Fixture {
        keychain: Keychain,
        _dir: tempfile::TempDir,
        _ui: InteractionOff,
        _lock: MutexGuard<'static, ()>,
    }

    fn fixture() -> Fixture {
        // Locals would drop in reverse declaration order. This struct drops
        // the keychain, then the directory, then restores prompts, then the lock.
        let lock = TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let ui = InteractionOff::enter().expect("disable keychain prompts");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sqlg-test.keychain-db");
        let keychain = Keychain::create(&path, "temp-pw").expect("temp keychain");
        Fixture {
            keychain,
            _dir: dir,
            _ui: ui,
            _lock: lock,
        }
    }

    #[test]
    fn open_acl_round_trip_with_prompts_disabled() {
        let fx = fixture();
        let account = "conn:open-round-trip";
        fx.keychain.set(account, "open-secret").expect("set");
        assert!(
            fx.keychain.access_is_open(account).expect("acl"),
            "new item is open: {}",
            fx.keychain.describe_access(account)
        );
        assert_eq!(
            fx.keychain.get(account).expect("get").as_deref(),
            Some("open-secret")
        );

        // The second read is served from the process cache, so removing the
        // item underneath does not change it. Forgetting the cache reads again.
        delete_item(&fx.keychain.keychain, account).expect("raw delete");
        assert_eq!(
            fx.keychain.get(account).expect("cached").as_deref(),
            Some("open-secret")
        );
        fx.keychain.forget_cached(account);
        assert_eq!(fx.keychain.get(account).expect("miss"), None);

        fx.keychain.set(account, "open-secret").expect("set again");
        fx.keychain.delete(account).expect("delete");
        assert_eq!(fx.keychain.get(account).expect("deleted"), None);
    }

    #[test]
    fn default_acl_is_rewritten_without_a_prompt() {
        let fx = fixture();
        let account = "conn:legacy-rewrite";
        add_default(&fx.keychain.keychain, account, "legacy-secret").expect("default acl");
        assert!(
            !fx.keychain.access_is_open(account).expect("acl"),
            "default acl is restrictive: {}",
            fx.keychain.describe_access(account)
        );

        assert_eq!(
            fx.keychain.get(account).expect("migrate").as_deref(),
            Some("legacy-secret")
        );
        assert!(
            fx.keychain.access_is_open(account).expect("acl"),
            "rewritten item is open: {}",
            fx.keychain.describe_access(account)
        );

        fx.keychain.forget_cached(account);
        assert_eq!(
            fx.keychain.get(account).expect("reread").as_deref(),
            Some("legacy-secret")
        );
        assert!(fx.keychain.access_is_open(account).expect("acl"));
    }

    #[test]
    fn set_replaces_a_restrictive_item_without_prompting() {
        let fx = fixture();
        let account = "conn:replace-restrictive";
        add_default(&fx.keychain.keychain, account, "old-secret").expect("default acl");
        fx.keychain.set(account, "replaced-secret").expect("set");
        assert!(
            fx.keychain.access_is_open(account).expect("acl"),
            "replaced item is open: {}",
            fx.keychain.describe_access(account)
        );
        fx.keychain.forget_cached(account);
        assert_eq!(
            fx.keychain.get(account).expect("get").as_deref(),
            Some("replaced-secret")
        );
    }
}
