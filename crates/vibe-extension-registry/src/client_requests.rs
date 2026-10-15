//! Pure projection of the incumbent selected mechanism registry.
use crate::{MechanismRegistry, MechanismRegistryRow};
use std::collections::BTreeMap;

pub fn client_requests(
    registry: &MechanismRegistry,
) -> Result<BTreeMap<String, &MechanismRegistryRow>, String> {
    let mut map = BTreeMap::new();
    for row in registry.rows() {
        if let Some(request) = &row.declaration().client_request {
            request.validate()?;
            if map.insert(request.client_id.clone(), row).is_some() {
                return Err(format!("duplicate client binding: {}", request.client_id));
            }
        }
    }
    Ok(map)
}
#[cfg(test)]
mod m_08_c_1_cases {
    use super::*;
    use crate::{
        ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_mechanisms,
    };
    use vibe_core::manifest::{ExtensionsControl, Manifest};
    #[test]
    fn selected_order_owner_disable_and_source_binding() {
        let text = "[project]\nname='fixture'\nversion='1.0.0'\n[[mechanism]]\nid='client'\nrole='deploy'\nname='client'\nhandler={kind='binary',name='unavailable'}\nprotocol=1\nconfig_schema='schema.json'\nfreshness='engine'\n[mechanism.client_request]\nschema=1\nclient_id='org.example/client'\nversion='*'\n[[mechanism.client_request.bindings]]\nscope='project'\nshape='file'\ndestination='.client/data'\nkey=[]\noperations=['project-file']\n";
        let manifest = Manifest::parse_str(text).unwrap();
        let mut world = ExtensionWorld {
            installed: vec![],
            effective_stack: None,
            host: HostExtensionSource {
                provider: HostProvider {
                    identity: HostIdentity::ungrouped_project("fixture"),
                    root: "/fixture".into(),
                    version: "1.0.0".into(),
                    kind: None,
                    content_hash: None,
                },
                declarations: vec![],
                controls: ExtensionsControl::default(),
                mechanisms: manifest.mechanism_decls,
            },
        };
        let registry = collect_mechanisms(&world).unwrap();
        let map = client_requests(&registry).unwrap();
        assert_eq!(
            map["org.example/client"].pin().to_string(),
            "__host__/fixture#client"
        );
        world
            .host
            .controls
            .disable
            .push(vibe_core::manifest::ExtensionKey::authored(
                "__host__/fixture#client",
            ));
        let disabled = collect_mechanisms(&world).unwrap();
        assert!(client_requests(&disabled).unwrap()["org.example/client"].is_disabled());
        let mut declaration = world.host.mechanisms[0].clone();
        declaration.client_request.as_mut().unwrap().client_id =
            "org.example/dependency-client".into();
        world.installed.push(crate::DependencyExtensionSource {
            provider: crate::DependencyProvider {
                id: crate::DependencyProviderId::new(
                    "org.example".parse().unwrap(),
                    "dependency".parse().unwrap(),
                ),
                root: "/selected/dependency".into(),
                version: "2.0.0".into(),
                kind: vibe_core::PackageKind::Tool,
                content_hash: vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64)))
                    .unwrap(),
            },
            declarations: vec![],
            controls: ExtensionsControl::default(),
            mechanisms: vec![declaration],
        });
        let registry = collect_mechanisms(&world).unwrap();
        let map = client_requests(&registry).unwrap();
        assert_eq!(
            map["org.example/dependency-client"].provider_ordinal(),
            Some(0)
        );
        assert_eq!(
            map["org.example/dependency-client"].provider(),
            &crate::MechanismProvider::Dependency(world.installed[0].provider.clone())
        );
        let mut duplicate = world.host.mechanisms[0].clone();
        duplicate.id = "other".into();
        world.host.mechanisms.push(duplicate);
        assert!(collect_mechanisms(&world).is_err());
    }
}
