//! Typed passes with object-safe storage for heterogeneous pipeline segments.
//!
//! Implementations see concrete input/output carriers and own their concrete
//! error type. The private adapter erases those three types; [`PassSegment`]
//! checks both sides of every erased call defensively.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;

use super::ir::{
    ClosureIr, DocumentIr, Documents, EmittedIr, IrCardinality, IrLevel, IrShape, LaneIr, SourceIr,
};
use super::verify::IrVerifier;

/// Stable identity used to position and attribute one compiler pass.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PassName(String);

impl PassName {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, PassNameError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(PassNameError)
        } else {
            Ok(Self(value))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A blank pass name cannot be attributed or positioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("compiler pass name must not be blank")]
pub(crate) struct PassNameError;

/// The type-level contract of one registered pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassDescriptor {
    pub(crate) name: PassName,
    pub(crate) input: IrShape,
    pub(crate) output: IrShape,
}

/// The six manager carriers over the five IR levels.
///
/// `Document` is one per-document value; `Documents` is the artifact batch at
/// the same level. Their cardinalities make the distinction executable.
#[derive(Debug)]
pub(crate) enum AnyIr {
    Source(SourceIr),
    Document(DocumentIr),
    Documents(Documents),
    Closure(ClosureIr),
    Lane(LaneIr),
    Emitted(EmittedIr),
}

impl AnyIr {
    pub(crate) const fn shape(&self) -> IrShape {
        match self {
            Self::Source(_) => IrShape::new(IrLevel::Source, IrCardinality::Document),
            Self::Document(_) => IrShape::new(IrLevel::Document, IrCardinality::Document),
            Self::Documents(_) => IrShape::new(IrLevel::Document, IrCardinality::Artifact),
            Self::Closure(_) => IrShape::new(IrLevel::Closure, IrCardinality::Artifact),
            Self::Lane(_) => IrShape::new(IrLevel::Lane, IrCardinality::Artifact),
            Self::Emitted(_) => IrShape::new(IrLevel::Emitted, IrCardinality::Artifact),
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

/// A concrete IR carrier allowed across the manager's erased boundary.
///
/// The private seal fixes this set to the six [`AnyIr`] variants. A downstream
/// module cannot manufacture a seventh carrier whose declared and erased
/// shapes disagree.
pub(crate) trait IrPayload: sealed::Sealed + Sized + Send + 'static {
    const SHAPE: IrShape;

    fn into_any(self) -> AnyIr;
    fn try_from_any(value: AnyIr) -> Result<Self, Box<AnyIr>>;
}

macro_rules! payload {
    ($type:ty, $variant:ident, $level:ident, $cardinality:ident) => {
        impl sealed::Sealed for $type {}

        impl IrPayload for $type {
            const SHAPE: IrShape = IrShape::new(IrLevel::$level, IrCardinality::$cardinality);

            fn into_any(self) -> AnyIr {
                AnyIr::$variant(self)
            }

            fn try_from_any(value: AnyIr) -> Result<Self, Box<AnyIr>> {
                match value {
                    AnyIr::$variant(value) => Ok(value),
                    other => Err(Box::new(other)),
                }
            }
        }
    };
}

payload!(SourceIr, Source, Source, Document);
payload!(DocumentIr, Document, Document, Document);
payload!(Documents, Documents, Document, Artifact);
payload!(ClosureIr, Closure, Closure, Artifact);
payload!(LaneIr, Lane, Lane, Artifact);
payload!(EmittedIr, Emitted, Emitted, Artifact);

/// One typed compiler pass. Wrong output carrier types are unrepresentable at
/// this surface; the erased segment still verifies the runtime shape.
pub(crate) trait Pass: Send + Sync + 'static {
    type Input: IrPayload;
    type Output: IrPayload;
    type Error: Error + Send + Sync + 'static;

    fn name(&self) -> &PassName;
    fn run(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub(crate) trait DynPass: Send + Sync {
    fn descriptor(&self) -> PassDescriptor;
    fn run_erased(&self, input: AnyIr) -> Result<AnyIr, PassSegmentError>;
}

struct ErasedPass<P>(P);

impl<P: Pass> DynPass for ErasedPass<P> {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor {
            name: self.0.name().clone(),
            input: P::Input::SHAPE,
            output: P::Output::SHAPE,
        }
    }

    fn run_erased(&self, input: AnyIr) -> Result<AnyIr, PassSegmentError> {
        let actual = input.shape();
        let input = P::Input::try_from_any(input).map_err(|_| PassSegmentError::WrongInput {
            pass: self.0.name().clone(),
            expected: P::Input::SHAPE,
            actual,
        })?;
        self.0
            .run(input)
            .map(IrPayload::into_any)
            .map_err(|source| PassSegmentError::PassFailed {
                pass: self.0.name().clone(),
                source: Box::new(source),
            })
    }
}

/// A validated heterogeneous linear segment of the declared schedule.
#[derive(Default)]
pub(crate) struct PassSegment {
    passes: Vec<Box<dyn DynPass>>,
}

impl PassSegment {
    pub(crate) fn push<P: Pass>(&mut self, pass: P) -> Result<(), PassSegmentError> {
        self.push_dyn(Box::new(ErasedPass(pass)))
    }

    fn push_dyn(&mut self, pass: Box<dyn DynPass>) -> Result<(), PassSegmentError> {
        let descriptor = pass.descriptor();
        if self
            .passes
            .iter()
            .any(|registered| registered.descriptor().name == descriptor.name)
        {
            return Err(PassSegmentError::DuplicateName {
                pass: descriptor.name,
            });
        }
        if let Some(previous) = self.passes.last().map(|pass| pass.descriptor())
            && previous.output != descriptor.input
        {
            return Err(PassSegmentError::BrokenChain {
                previous: previous.name,
                previous_output: previous.output,
                next: descriptor.name,
                next_input: descriptor.input,
            });
        }
        self.passes.push(pass);
        Ok(())
    }

    /// Test-only route past the typed pass surface, so the erased manager's
    /// own defensive checks (wrong runtime output carrier) stay provable.
    /// Production passes enter through [`Self::push`].
    #[cfg(test)]
    pub(crate) fn push_erased_for_test(
        &mut self,
        pass: Box<dyn DynPass>,
    ) -> Result<(), PassSegmentError> {
        self.push_dyn(pass)
    }

    pub(crate) fn descriptors(&self) -> impl Iterator<Item = PassDescriptor> + '_ {
        self.passes.iter().map(|pass| pass.descriptor())
    }

    pub(crate) fn first_input(&self) -> Option<IrShape> {
        self.passes.first().map(|pass| pass.descriptor().input)
    }

    pub(crate) fn last_output(&self) -> Option<IrShape> {
        self.passes.last().map(|pass| pass.descriptor().output)
    }

    pub(crate) fn run(&self, input: AnyIr) -> Result<AnyIr, PassSegmentError> {
        self.run_checked(input, None)
    }

    /// The one execution path shared by every heterogeneous pass. When a
    /// verifier is present (R3.3: `#[cfg(test)]` only), the segment input is
    /// verified at its honest engine/gather boundary before any pass runs, and
    /// every successful, correctly shaped pass output is semantically verified
    /// and authenticated against an immutable pre-pass witness before the next
    /// pass is invoked (PROP-054 `##INTER-PASS-VERIFIER`).
    pub(crate) fn run_checked(
        &self,
        mut input: AnyIr,
        verifier: Option<IrVerifier>,
    ) -> Result<AnyIr, PassSegmentError> {
        if let Some(verifier) = verifier {
            verifier
                .verify(&input)
                .map_err(|source| PassSegmentError::InputVerification {
                    input: input.shape(),
                    source: Box::new(source),
                })?;
        }
        for pass in &self.passes {
            let descriptor = pass.descriptor();
            let before = verifier
                .map(|verifier| verifier.witness(&input))
                .transpose()
                .map_err(|source| PassSegmentError::InputVerification {
                    input: input.shape(),
                    source: Box::new(source),
                })?;
            input = pass.run_erased(input)?;
            let actual = input.shape();
            if actual != descriptor.output {
                return Err(PassSegmentError::WrongOutput {
                    pass: descriptor.name,
                    expected: descriptor.output,
                    actual,
                });
            }
            if let Some(verifier) = verifier {
                verifier
                    .verify(&input)
                    .and_then(|()| match &before {
                        Some(before) => verifier.verify_transition(before, &input),
                        None => Ok(()),
                    })
                    .map_err(|source| PassSegmentError::VerificationFailed {
                        pass: descriptor.name,
                        output: actual,
                        source: Box::new(source),
                    })?;
            }
        }
        Ok(input)
    }
}

/// Why a pass segment or invocation violated its typed contract.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PassSegmentError {
    #[error("duplicate compiler pass name `{pass}`")]
    DuplicateName { pass: PassName },
    #[error(
        "compiler pass `{next}` expects {next_input:?}, but previous pass `{previous}` returns {previous_output:?}"
    )]
    BrokenChain {
        previous: PassName,
        previous_output: IrShape,
        next: PassName,
        next_input: IrShape,
    },
    #[error("compiler pass `{pass}` expects {expected:?}, got {actual:?}")]
    WrongInput {
        pass: PassName,
        expected: IrShape,
        actual: IrShape,
    },
    #[error("compiler pass `{pass}` returned {actual:?}, but declared {expected:?}")]
    WrongOutput {
        pass: PassName,
        expected: IrShape,
        actual: IrShape,
    },
    #[error("compiler pass `{pass}` failed: {source}")]
    PassFailed {
        pass: PassName,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
    #[error(
        "compiler segment input {input:?} failed semantic verification before any pass ran: {source}"
    )]
    InputVerification {
        input: IrShape,
        #[source]
        source: Box<super::verify::VerificationError>,
    },
    #[error("compiler pass `{pass}` returned semantically invalid {output:?} IR: {source}")]
    VerificationFailed {
        pass: PassName,
        output: IrShape,
        #[source]
        source: Box<super::verify::VerificationError>,
    },
}

/// A typed no-op used to prove the erased manager preserves an IR value.
pub(crate) struct IdentityPass<T> {
    name: PassName,
    marker: PhantomData<fn(T) -> T>,
}

impl<T> IdentityPass<T> {
    pub(crate) fn new(name: PassName) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }
}

impl<T: IrPayload> Pass for IdentityPass<T> {
    type Input = T;
    type Output = T;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: T) -> Result<T, Infallible> {
        Ok(input)
    }
}

#[cfg(test)]
mod tests;
