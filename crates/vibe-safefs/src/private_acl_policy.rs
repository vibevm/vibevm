//! Test-only policy over normalized facts, NOT a Windows security-descriptor
//! parser, AccessCheck replacement, held-handle witness or private-write gate.
//! Rights are semantic labels, never Win32 numeric access masks. A future
//! normalizer must independently prove SID, ACE, mask and descriptor validity.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    File,
    Directory,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Right {
    Read,
    Write,
    Append,
    Execute,
    Delete,
    ReadAttributes,
    WriteAttributes,
    ReadExtendedAttributes,
    WriteExtendedAttributes,
    ReadControl,
    WriteDacl,
    WriteOwner,
    Synchronize,
    List,
    AddFile,
    AddDirectory,
    Traverse,
    DeleteChild,
    Unknown,
}
#[derive(Clone, Copy)]
enum Ace<'a> {
    // Zero flags is the only admitted form: inherited and propagation flags
    // are not silently interpreted or stripped by this policy.
    Allow {
        trustee: &'a str,
        flags: u8,
        rights: &'a [Right],
    },
    Unsupported,
}
enum Dacl<'a> {
    Absent,
    Null,
    Present(&'a [Ace<'a>]),
}
struct Descriptor<'a> {
    // Opaque normalized SID identities, not textual SID parsing. Their
    // canonical validity is a precondition belonging to a future normalizer.
    owner: &'a str,
    protected: bool,
    unsupported_control: bool,
    dacl: Dacl<'a>,
}
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Owner,
    Control,
    Dacl,
    Ace,
    Rights,
    MissingRights,
}
fn classify(
    current_sid: &str,
    profile: Profile,
    descriptor: &Descriptor<'_>,
) -> Result<(), Refusal> {
    use Right::*;
    if current_sid.is_empty() || descriptor.owner != current_sid {
        return Err(Refusal::Owner);
    }
    if !descriptor.protected || descriptor.unsupported_control {
        return Err(Refusal::Control);
    }
    let Dacl::Present(aces) = descriptor.dacl else {
        return Err(Refusal::Dacl);
    };
    if aces.is_empty() {
        return Err(Refusal::Dacl);
    }
    let common = [
        Delete,
        ReadAttributes,
        WriteAttributes,
        ReadExtendedAttributes,
        WriteExtendedAttributes,
        ReadControl,
        WriteDacl,
        WriteOwner,
        Synchronize,
    ];
    let specific: &[Right] = match profile {
        Profile::File => &[Read, Write, Append, Execute],
        Profile::Directory => &[List, AddFile, AddDirectory, Traverse, DeleteChild],
    };
    let required: &[Right] = match profile {
        Profile::File => &[Read, Write, ReadAttributes, ReadControl, Synchronize],
        Profile::Directory => &[
            List,
            AddFile,
            AddDirectory,
            Traverse,
            ReadAttributes,
            ReadControl,
            Synchronize,
        ],
    };
    let mut granted = Vec::new();
    for ace in aces {
        let Ace::Allow {
            trustee,
            flags: 0,
            rights,
        } = ace
        else {
            return Err(Refusal::Ace);
        };
        if *trustee != current_sid {
            return Err(Refusal::Ace);
        }
        if rights.is_empty()
            || rights
                .iter()
                .any(|right| !common.contains(right) && !specific.contains(right))
        {
            return Err(Refusal::Rights);
        }
        granted.extend_from_slice(rights);
    }
    if required.iter().any(|right| !granted.contains(right)) {
        return Err(Refusal::MissingRights);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Right::*;
    const USER: &str = "normalized-current-sid";
    const FILE: &[Right] = &[Read, Write, ReadAttributes, ReadControl, Synchronize];
    const DIR: &[Right] = &[
        List,
        AddFile,
        AddDirectory,
        Traverse,
        ReadAttributes,
        ReadControl,
        Synchronize,
    ];
    fn descriptor<'a>(aces: &'a [Ace<'a>]) -> Descriptor<'a> {
        Descriptor {
            owner: USER,
            protected: true,
            unsupported_control: false,
            dacl: Dacl::Present(aces),
        }
    }
    #[test]
    fn explicit_current_owner_private_profiles_and_split_grants_pass() {
        for (profile, rights) in [(Profile::File, FILE), (Profile::Directory, DIR)] {
            let ace = [Ace::Allow {
                trustee: USER,
                flags: 0,
                rights,
            }];
            assert_eq!(classify(USER, profile, &descriptor(&ace)), Ok(()));
        }
        let aces = [
            Ace::Allow {
                trustee: USER,
                flags: 0,
                rights: &FILE[..2],
            },
            Ace::Allow {
                trustee: USER,
                flags: 0,
                rights: &FILE[2..],
            },
        ];
        assert_eq!(classify(USER, Profile::File, &descriptor(&aces)), Ok(()));
    }
    #[test]
    fn owner_and_descriptor_control_refuse_closed() {
        let aces = [Ace::Allow {
            trustee: USER,
            flags: 0,
            rights: FILE,
        }];
        let mut d = descriptor(&aces);
        d.owner = "foreign-sid";
        assert_eq!(classify(USER, Profile::File, &d), Err(Refusal::Owner));
        d.owner = USER;
        assert_eq!(classify("", Profile::File, &d), Err(Refusal::Owner));
        d.protected = false;
        assert_eq!(classify(USER, Profile::File, &d), Err(Refusal::Control));
        d.protected = true;
        d.unsupported_control = true;
        assert_eq!(classify(USER, Profile::File, &d), Err(Refusal::Control));
    }
    #[test]
    fn absent_null_empty_and_unknown_ace_forms_refuse() {
        for dacl in [Dacl::Absent, Dacl::Null, Dacl::Present(&[])] {
            let mut d = descriptor(&[]);
            d.dacl = dacl;
            assert_eq!(classify(USER, Profile::File, &d), Err(Refusal::Dacl));
        }
        let aces = [Ace::Unsupported];
        assert_eq!(
            classify(USER, Profile::File, &descriptor(&aces)),
            Err(Refusal::Ace)
        );
    }
    #[test]
    fn foreign_broad_grants_and_every_inheritance_flag_refuse() {
        for trustee in [
            "everyone-sid",
            "authenticated-users-sid",
            "administrators-sid",
            "system-sid",
            "other-user-sid",
            "",
        ] {
            let aces = [Ace::Allow {
                trustee,
                flags: 0,
                rights: FILE,
            }];
            assert_eq!(
                classify(USER, Profile::File, &descriptor(&aces)),
                Err(Refusal::Ace)
            );
        }
        for flags in 1..=u8::MAX {
            let aces = [Ace::Allow {
                trustee: USER,
                flags,
                rights: FILE,
            }];
            assert_eq!(
                classify(USER, Profile::File, &descriptor(&aces)),
                Err(Refusal::Ace)
            );
        }
    }
    #[test]
    fn unknown_empty_insufficient_and_cross_profile_rights_refuse() {
        for rights in [&[][..], &[Unknown][..], &[Read][..], DIR] {
            let aces = [Ace::Allow {
                trustee: USER,
                flags: 0,
                rights,
            }];
            assert!(classify(USER, Profile::File, &descriptor(&aces)).is_err());
        }
        let aces = [Ace::Allow {
            trustee: USER,
            flags: 0,
            rights: FILE,
        }];
        assert_eq!(
            classify(USER, Profile::Directory, &descriptor(&aces)),
            Err(Refusal::Rights)
        );
        let mut all = FILE.to_vec();
        all.extend_from_slice(&[
            Append,
            Execute,
            Delete,
            WriteAttributes,
            ReadExtendedAttributes,
            WriteExtendedAttributes,
            WriteDacl,
            WriteOwner,
        ]);
        let aces = [Ace::Allow {
            trustee: USER,
            flags: 0,
            rights: &all,
        }];
        assert_eq!(classify(USER, Profile::File, &descriptor(&aces)), Ok(()));
        let mut all = DIR.to_vec();
        all.push(DeleteChild);
        let aces = [Ace::Allow {
            trustee: USER,
            flags: 0,
            rights: &all,
        }];
        assert_eq!(
            classify(USER, Profile::Directory, &descriptor(&aces)),
            Ok(())
        );
    }
}
