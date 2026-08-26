//! Strict serde intermediates for the authored TOML `[[extension]]` table.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR");

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::{
    ExtensionAppliesTo, ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionIrLevel,
    ExtensionKey, ExtensionPass, ExtensionPassKind, ExtensionUse, ExtensionWhen, ExtensionsControl,
};
use crate::lifecycle::ExtensionPoint;

const POINT_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtensionDeclWire {
    id: String,
    point: String,
    handler: ExtensionHandlerWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<toml::Table>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auto: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inputs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    applies_to: Option<ExtensionAppliesToWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compiler_internals: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pass: Option<ExtensionPassWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<toml::Table>,
}

/// Strict wire shape for the plural consumer-side `[extensions]` namespace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
pub(crate) struct ExtensionsControlWire {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    disable: Vec<String>,
    #[serde(default, rename = "use", skip_serializing_if = "Vec::is_empty")]
    uses: Vec<ExtensionUseWire>,
}

impl ExtensionsControlWire {
    pub(crate) fn is_empty(&self) -> bool {
        self.disable.is_empty() && self.uses.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionUseWire {
    #[serde(rename = "ref")]
    reference: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<toml::Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub(crate) enum ExtensionHandlerWire {
    Builtin {
        name: String,
    },
    Script {
        base: PathBuf,
    },
    Binary {
        name: String,
    },
    Native {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        crate_dir: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prebuilt: Option<BTreeMap<String, PathBuf>>,
    },
    Agent {
        prompt: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionAppliesToWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    packages: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionPassWire {
    kind: ExtensionPassKindWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    level: Option<ExtensionIrLevelWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from: Option<ExtensionIrLevelWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    to: Option<ExtensionIrLevelWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    formats: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    artifact: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ExtensionPassKindWire {
    Transform,
    Lowering,
    Frontend,
    Backend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ExtensionIrLevelWire {
    Source,
    Document,
    Closure,
    Lane,
    Emitted,
}

impl TryFrom<ExtensionDeclWire> for ExtensionDecl {
    type Error = String;

    fn try_from(wire: ExtensionDeclWire) -> Result<Self, Self::Error> {
        let point = wire.point.parse::<ExtensionPoint>().map_err(|error| {
            format!(
                "[[extension]] `{}` field `point` value `{}` is invalid: {error} ({POINT_GRAMMAR})",
                wire.id, wire.point,
            )
        })?;
        let extension = Self {
            id: wire.id,
            point,
            handler: wire.handler.into(),
            config: wire.config.map(ExtensionConfig::from_table),
            auto: wire.auto,
            inputs: wire.inputs,
            applies_to: wire.applies_to.map(Into::into),
            compiler_internals: wire.compiler_internals,
            pass: wire.pass.map(Into::into),
            when: wire.when.map(ExtensionWhen::from_table),
        };
        Ok(extension)
    }
}

impl TryFrom<ExtensionDecl> for ExtensionDeclWire {
    type Error = String;

    fn try_from(extension: ExtensionDecl) -> Result<Self, Self::Error> {
        extension.validate()?;
        Ok(Self {
            id: extension.id,
            point: extension.point.to_string(),
            handler: extension.handler.into(),
            config: extension.config.map(ExtensionConfig::into_table),
            auto: extension.auto,
            inputs: extension.inputs,
            applies_to: extension.applies_to.map(Into::into),
            compiler_internals: extension.compiler_internals,
            pass: extension.pass.map(Into::into),
            when: extension.when.map(ExtensionWhen::into_table),
        })
    }
}

impl From<ExtensionsControlWire> for ExtensionsControl {
    fn from(wire: ExtensionsControlWire) -> Self {
        Self {
            uses: wire
                .uses
                .into_iter()
                .map(|entry| ExtensionUse {
                    reference: ExtensionKey::authored(entry.reference),
                    config: entry.config.map(ExtensionConfig::from_table),
                })
                .collect(),
            disable: wire
                .disable
                .into_iter()
                .map(ExtensionKey::authored)
                .collect(),
        }
    }
}

impl From<ExtensionsControl> for ExtensionsControlWire {
    fn from(controls: ExtensionsControl) -> Self {
        Self {
            disable: controls
                .disable
                .into_iter()
                .map(|key| key.as_str().to_owned())
                .collect(),
            uses: controls
                .uses
                .into_iter()
                .map(|entry| ExtensionUseWire {
                    reference: entry.reference.as_str().to_owned(),
                    config: entry.config.map(ExtensionConfig::into_table),
                })
                .collect(),
        }
    }
}

impl From<ExtensionHandlerWire> for ExtensionHandler {
    fn from(wire: ExtensionHandlerWire) -> Self {
        match wire {
            ExtensionHandlerWire::Builtin { name } => Self::Builtin { name },
            ExtensionHandlerWire::Script { base } => Self::Script { base },
            ExtensionHandlerWire::Binary { name } => Self::Binary { name },
            ExtensionHandlerWire::Native {
                crate_dir,
                prebuilt,
            } => Self::Native {
                crate_dir,
                prebuilt,
            },
            ExtensionHandlerWire::Agent { prompt } => Self::Agent { prompt },
        }
    }
}

impl From<ExtensionHandler> for ExtensionHandlerWire {
    fn from(handler: ExtensionHandler) -> Self {
        match handler {
            ExtensionHandler::Builtin { name } => Self::Builtin { name },
            ExtensionHandler::Script { base } => Self::Script { base },
            ExtensionHandler::Binary { name } => Self::Binary { name },
            ExtensionHandler::Native {
                crate_dir,
                prebuilt,
            } => Self::Native {
                crate_dir,
                prebuilt,
            },
            ExtensionHandler::Agent { prompt } => Self::Agent { prompt },
        }
    }
}

impl From<ExtensionAppliesToWire> for ExtensionAppliesTo {
    fn from(wire: ExtensionAppliesToWire) -> Self {
        Self {
            packages: wire.packages,
            paths: wire.paths,
        }
    }
}

impl From<ExtensionAppliesTo> for ExtensionAppliesToWire {
    fn from(selector: ExtensionAppliesTo) -> Self {
        Self {
            packages: selector.packages,
            paths: selector.paths,
        }
    }
}

impl From<ExtensionPassWire> for ExtensionPass {
    fn from(wire: ExtensionPassWire) -> Self {
        Self {
            kind: wire.kind.into(),
            level: wire.level.map(Into::into),
            from: wire.from.map(Into::into),
            to: wire.to.map(Into::into),
            after: wire.after,
            before: wire.before,
            replace: wire.replace,
            formats: wire.formats,
            artifact: wire.artifact,
        }
    }
}

impl From<ExtensionPass> for ExtensionPassWire {
    fn from(pass: ExtensionPass) -> Self {
        Self {
            kind: pass.kind.into(),
            level: pass.level.map(Into::into),
            from: pass.from.map(Into::into),
            to: pass.to.map(Into::into),
            after: pass.after,
            before: pass.before,
            replace: pass.replace,
            formats: pass.formats,
            artifact: pass.artifact,
        }
    }
}

impl From<ExtensionPassKindWire> for ExtensionPassKind {
    fn from(kind: ExtensionPassKindWire) -> Self {
        match kind {
            ExtensionPassKindWire::Transform => Self::Transform,
            ExtensionPassKindWire::Lowering => Self::Lowering,
            ExtensionPassKindWire::Frontend => Self::Frontend,
            ExtensionPassKindWire::Backend => Self::Backend,
        }
    }
}

impl From<ExtensionPassKind> for ExtensionPassKindWire {
    fn from(kind: ExtensionPassKind) -> Self {
        match kind {
            ExtensionPassKind::Transform => Self::Transform,
            ExtensionPassKind::Lowering => Self::Lowering,
            ExtensionPassKind::Frontend => Self::Frontend,
            ExtensionPassKind::Backend => Self::Backend,
        }
    }
}

impl From<ExtensionIrLevelWire> for ExtensionIrLevel {
    fn from(level: ExtensionIrLevelWire) -> Self {
        match level {
            ExtensionIrLevelWire::Source => Self::Source,
            ExtensionIrLevelWire::Document => Self::Document,
            ExtensionIrLevelWire::Closure => Self::Closure,
            ExtensionIrLevelWire::Lane => Self::Lane,
            ExtensionIrLevelWire::Emitted => Self::Emitted,
        }
    }
}

impl From<ExtensionIrLevel> for ExtensionIrLevelWire {
    fn from(level: ExtensionIrLevel) -> Self {
        match level {
            ExtensionIrLevel::Source => Self::Source,
            ExtensionIrLevel::Document => Self::Document,
            ExtensionIrLevel::Closure => Self::Closure,
            ExtensionIrLevel::Lane => Self::Lane,
            ExtensionIrLevel::Emitted => Self::Emitted,
        }
    }
}
