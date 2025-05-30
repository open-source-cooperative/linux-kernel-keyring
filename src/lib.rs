/*!
# Credential store for the keyring crate

This module implements a credential store for the keyring crate that uses keyutils as its back end.

## Usage

If you are trying to use the keyring crate on a headless linux box, or one that doesn't come with
gnome-keyring, it's strongly recommended that you use this credential store, because (as part of the kernel)
it's always available on Linux.

To make this keystore the default for creation of keyring entries, execute this code:
```
keyring::set_default_credential_builder(linux_kernel_keyring::KeyutilsCredentialBuilder::new())
```

# Attributes

Entries in keyutils are identified by a string `description`.  If a keyring entry is created with
an explicit `target`, that value is used as the keyutils description.  Otherwise, the string
`keyring:user@service` is used (where user and service come from the entry creation call).

There is no notion of attribute other than the description supported by keyutils,
so the [get_attributes](keyring::Entry::get_attributes)
and [update_attributes](keyring::Entry::update_attributes)
calls are both no-ops for this credential store.

# Persistence

The key management facility provided by the kernel is completely in-memory and will not persist
across reboots. Consider the keyring a secure cache and plan for your application to handle
cases where the entry is no longer available in-memory.

In other words, you should prepare for `Entry::get_password` to fail and have a fallback to re-load
the credential into memory.

Potential options to re-load the credential into memory are:

- Re-prompt the user (most common/effective for CLI applications)
- Create a PAM module or use `pam_exec` to load a credential securely when the user logs in.
- If you're running as a systemd service you can use `systemd-ask-password` to prompt the user
  when your service starts.
  
```
use std::error::Error;
use keyring::Entry;

/// Simple user code that handles retrieving a credential regardless
/// of the credential state.
struct CredentialManager {
    entry: Entry,
}

impl CredentialManager {
    /// Init the service as normal
    pub fn new(service: &str, user: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            entry: Entry::new(service, user)?
        })
    }

    /// Method that first attempts to retrieve the credential from memory
    /// and falls back to prompting the user.
    pub fn get(&self) -> Result<String, Box<dyn Error>> {
        self.entry.get_password().or_else(|_| self.prompt())
    }

    /// Internal method to prompt the user and cache the credential
    /// in memory for subsequent lookups.
    fn prompt(&self) -> Result<String, Box<dyn Error>> {
        let password = rpassword::read_password()?;
        self.entry.set_password(&password)?;
        Ok(password)
    }
}
```

A single entry in keyutils can be on multiple "keyrings", each of which has a subtly
different lifetime.  The core storage for keyring keys is provided by the user-specific
[persistent keyring](https://www.man7.org/linux/man-pages/man7/persistent-keyring.7.html),
whose lifetime defaults to a few days (and is controllable by
administrators).  But whenever an entry's credential is used,
it is also added to the user's
[session keyring](https://www.man7.org/linux/man-pages/man7/session-keyring.7.html):
this ensures that the credential will persist as long as the user session exists, and when the user
logs out the credential will persist as long as the persistent keyring doesn't expire while the user is
logged out.

Each time the `Entry::new()` operation is performed, the persistent keyring's expiration timer
is reset to the value configured in:

```no_run,no_test,ignore
proc/sys/kernel/keys/persistent_keyring_expiry
```

| Persistent Keyring State | Session Keyring State | User Key State |
| -------------            | -------------         | -------------  |
| Active                   | Active                | Active         |
| Expired                  | Active                | Active         |
| Active                   | Logged Out            | Active (Accessible on next login)        |
| Expired                  | Logged Out            | Expired        |

**Note**: As mentioned above, a reboot clears all keyrings.
*/
mod error;

mod credentials;
pub use credentials::KeyutilsCredential;

mod builder;
pub use builder::KeyutilsCredentialBuilder;

#[cfg(test)]
mod tests;
