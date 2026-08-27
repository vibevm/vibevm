//! Emitted-artifact conversion: opaque bytes, digests, and the EMIT IDENTITY
//! gate. This is also the home of the shared digest-hex codec.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::super::backend::BackendId;
use super::super::emit::emitted_bytes_digest;
use super::super::ir::{
    ArtifactContext, ArtifactFrame, COMPATIBILITY_ARTIFACT_ID, ClosureNodeId,
    EmissionContributionWitness, EmissionProvenance, EmittedArtifact, LaneInputDigest,
    OriginRename,
};
use super::super::pass::PassName;
use super::address::{
    decode_document_address, decode_spec_address, encode_document_address, encode_spec_address,
};
// Every spelling/identity diagnostic here rides the one bounded discipline.
use super::bounded::preview as bounded_preview;
use super::closure::{
    apply_package_relation, decode_context, decode_meta, encode_context, encode_meta,
};
use super::{
    G_DIGEST_BASE64, G_EMIT_IDENTITY, G_SCALAR_IDS, IrWireError, gate, narrow, require_scalar,
    widen, wire,
};

/// The framing cell names this type so its refusals stay `emit-identity`.
pub(super) type GateRefusal = IrWireError;

pub(super) fn gate_emit_identity(detail: impl Into<String>) -> GateRefusal {
    gate(G_EMIT_IDENTITY, detail)
}

/// 64 lowercase hex characters — validated BEFORE any parse allocates from it.
pub(super) fn parse_digest(field: &'static str, value: &str) -> Result<[u8; 32], IrWireError> {
    let bytes = value.as_bytes();
    if bytes.len() != 64
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        // The preview is built only on the failure path; valid input pays no
        // allocation here.
        return Err(gate(
            G_DIGEST_BASE64,
            format!(
                "{field} must be 64 lowercase hex characters, got {}",
                bounded_preview(value)
            ),
        ));
    }
    let mut digest = [0u8; 32];
    for (slot, pair) in digest.iter_mut().zip(bytes.chunks_exact(2)) {
        *slot = pair
            .iter()
            .fold(0u8, |acc, byte| (acc << 4) | hex_nibble(*byte));
    }
    Ok(digest)
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        _ => byte - b'a' + 10,
    }
}

pub(super) fn digest_hex(value: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(char::from_digit(u32::from(*byte >> 4), 16).expect("low nibble is hex"));
        out.push(char::from_digit(u32::from(*byte & 0x0f), 16).expect("high nibble is hex"));
    }
    out
}

/// Canonical padded STANDARD base64, checked WITHOUT allocating from the
/// decoded length: alphabet, length/padding, and zero trailing bits. Only a
/// canonical spelling reaches the audited engine, which then decodes once.
pub(super) fn decode_base64(value: &str) -> Result<Vec<u8>, IrWireError> {
    check_canonical_base64(value)?;
    STANDARD
        .decode(value)
        .map_err(|source| refuse_base64(&super::bounded::display(source)))
}

fn refuse_base64(detail: &str) -> IrWireError {
    gate(
        G_DIGEST_BASE64,
        format!("emitted bytes are not canonical padded standard base64: {detail}"),
    )
}

fn sextet(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub(super) fn check_canonical_base64(value: &str) -> Result<(), IrWireError> {
    // Zero proportional allocation on the success path: every quad is judged
    // by direct indexing and sextet checks; a preview is built only when a
    // fault is reported.
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Ok(()); // the canonical spelling of zero bytes
    }
    if !bytes.len().is_multiple_of(4) {
        return Err(refuse_base64(&format!(
            "length {} is not a multiple of four ({})",
            bytes.len(),
            bounded_preview(value)
        )));
    }
    let quads = bytes.len() / 4;
    for index in 0..quads {
        let quad = index * 4;
        let last = index + 1 == quads;
        let pad2 = last && bytes[quad + 2] == b'=' && bytes[quad + 3] == b'=';
        let pad1 = last && !pad2 && bytes[quad + 3] == b'=';
        if bytes[quad..quad + 4].contains(&b'=') && !pad1 && !pad2 {
            return Err(refuse_base64(&format!(
                "padding outside the final quad ({})",
                bounded_preview(value)
            )));
        }
        let data_len = 4 - usize::from(pad1) - 2 * usize::from(pad2);
        for slot in 0..data_len {
            if sextet(bytes[quad + slot]).is_none() {
                return Err(refuse_base64(&format!(
                    "byte 0x{:02x} is outside the standard alphabet ({})",
                    bytes[quad + slot],
                    bounded_preview(value)
                )));
            }
        }
        if pad2 && sextet(bytes[quad + 1]).unwrap_or(1) & 0x0f != 0 {
            return Err(refuse_base64(&format!(
                "non-zero trailing bits ({})",
                bounded_preview(value)
            )));
        }
        if pad1 && sextet(bytes[quad + 2]).unwrap_or(1) & 0x03 != 0 {
            return Err(refuse_base64(&format!(
                "non-zero trailing bits ({})",
                bounded_preview(value)
            )));
        }
    }
    Ok(())
}

pub(super) fn encode_base64(value: &[u8]) -> String {
    STANDARD.encode(value)
}

pub(super) fn decode_emitted(
    value: &wire::EmittedArtifact,
) -> Result<EmittedArtifact, IrWireError> {
    let provenance = &value.provenance;
    require_scalar("emit backend id", &provenance.backend)?;
    let backend = BackendId::new(provenance.backend.clone()).map_err(|_| {
        gate(
            G_SCALAR_IDS,
            format!(
                "emit backend id ({}) is refused by the id charset",
                bounded_preview(&provenance.backend)
            ),
        )
    })?;
    require_scalar("emit producer pass", &provenance.producer)?;
    let producer = PassName::new(provenance.producer.clone())
        .map_err(|_| gate(G_SCALAR_IDS, "emit producer pass must not be blank"))?;
    let context = decode_context(&provenance.context)?;
    let source_lane_digest = parse_digest("source lane digest", &provenance.source_lane_digest)?;
    let bytes_digest = parse_digest("bytes digest", &provenance.bytes_digest)?;
    let mut renames = Vec::with_capacity(provenance.renames.len());
    for rename in &provenance.renames {
        renames.push(OriginRename {
            origin: rename.origin.clone(),
            rename: crate::RenameEntry {
                original: rename.rename.original.clone(),
                qualified: rename.rename.qualified.clone(),
            },
        });
    }
    let mut contributions = Vec::with_capacity(provenance.contributions.len());
    for witness in &provenance.contributions {
        contributions.push(decode_emission_witness(witness)?);
    }
    let bytes = decode_base64(&value.bytes_b64)?;
    let provenance = EmissionProvenance {
        context,
        backend,
        producer,
        source_lane_digest: LaneInputDigest(source_lane_digest),
        renames,
        contributions,
        bytes_digest,
    };
    check_emit_identity(&provenance, &bytes)?;
    // Builtin tapes carry the engine's own header/marker framing; a custom
    // target under the compatibility frame stays opaque (identity + digest).
    if !provenance.context.target().is_custom() {
        super::framing::builtin(&provenance, &bytes)?;
    }
    Ok(EmittedArtifact { provenance, bytes })
}

/// The artifact id the carrier's own context ROW rides beside its backend.
/// Two of the three established rows spell it as the backend id; the BUILTIN
/// compatibility fragment is the reserved third — `static-fragment` rendered
/// by the `static-md` target/backend. This is not an implementation-only
/// exception: the tuple gate accepted exactly these rows, and gate 15 reads
/// the same law rather than a stricter one of its own.
fn expected_artifact_id<'a>(context: &ArtifactContext, backend: &'a str) -> &'a str {
    match (context.frame(), context.target().is_custom()) {
        (ArtifactFrame::CompatibilityFragment, false) => COMPATIBILITY_ARTIFACT_ID,
        _ => backend,
    }
}

/// EMIT IDENTITY: `target` and `backend` are one id, the artifact id is the
/// one this context row rides, `producer` is `emit:<backend>`, and
/// `bytes_digest` is the one digest recomputed from the wire document alone.
fn check_emit_identity(provenance: &EmissionProvenance, bytes: &[u8]) -> Result<(), IrWireError> {
    let backend = provenance.backend.as_str();
    let context = &provenance.context;
    let target = context.target();
    if target.backend_id() != backend {
        return Err(gate(
            G_EMIT_IDENTITY,
            format!(
                "target {} and backend {} are not one id",
                bounded_preview(target.backend_id()),
                bounded_preview(backend)
            ),
        ));
    }
    let expected_artifact = expected_artifact_id(context, backend);
    if context.artifact().as_str() != expected_artifact {
        return Err(gate(
            G_EMIT_IDENTITY,
            format!(
                "artifact {} is not the `{expected_artifact}` this context row rides",
                bounded_preview(context.artifact().as_str())
            ),
        ));
    }
    let expected_producer = format!("emit:{backend}");
    if provenance.producer.as_str() != expected_producer {
        return Err(gate(
            G_EMIT_IDENTITY,
            format!(
                "producer must be `{expected_producer}`, got {}",
                bounded_preview(provenance.producer.as_str())
            ),
        ));
    }
    let recomputed = emitted_bytes_digest(bytes);
    if recomputed != provenance.bytes_digest {
        return Err(gate(
            G_EMIT_IDENTITY,
            "bytes_digest is not the manager's independent digest of the carried bytes",
        ));
    }
    Ok(())
}

fn decode_emission_witness(
    value: &wire::EmissionContributionWitness,
) -> Result<EmissionContributionWitness, IrWireError> {
    Ok(match value {
        wire::EmissionContributionWitness::Normal(normal) => {
            let meta = decode_meta(&normal.meta)?;
            let seed_address = decode_spec_address(&normal.seed_address)?;
            apply_package_relation("normal", &meta, &seed_address, false)?;
            EmissionContributionWitness::Normal {
                meta,
                seed: ClosureNodeId(narrow("witness seed", normal.seed)?),
                seed_address,
                chunk_digest: parse_digest("chunk digest", &normal.chunk_digest)?,
            }
        }
        wire::EmissionContributionWitness::Simple(simple) => EmissionContributionWitness::Simple {
            meta: decode_meta(&simple.meta)?,
            address: decode_document_address(&simple.address)?,
            chunk_digest: parse_digest("chunk digest", &simple.chunk_digest)?,
        },
        wire::EmissionContributionWitness::Elided(elided) => EmissionContributionWitness::Elided {
            meta: decode_meta(&elided.meta)?,
        },
        wire::EmissionContributionWitness::Hoisted(hoisted) => {
            let meta = decode_meta(&hoisted.meta)?;
            let target = decode_spec_address(&hoisted.target)?;
            apply_package_relation("hoisted", &meta, &target, true)?;
            EmissionContributionWitness::Hoisted { meta, target }
        }
    })
}

pub(super) fn encode_emitted(
    value: &EmittedArtifact,
) -> Result<wire::EmittedArtifact, IrWireError> {
    let provenance = value.provenance();
    let mut contributions = Vec::with_capacity(provenance.contributions.len());
    for witness in &provenance.contributions {
        contributions.push(match witness {
            EmissionContributionWitness::Normal {
                meta,
                seed,
                seed_address,
                chunk_digest,
            } => wire::EmissionContributionWitness::Normal(Box::new(
                wire::EmissionContributionWitnessNormal {
                    meta: encode_meta(meta),
                    seed: widen("witness seed", seed.0)?,
                    seed_address: encode_spec_address(seed_address),
                    chunk_digest: digest_hex(chunk_digest),
                },
            )),
            EmissionContributionWitness::Simple {
                meta,
                address,
                chunk_digest,
            } => wire::EmissionContributionWitness::Simple(Box::new(
                wire::EmissionContributionWitnessSimple {
                    meta: encode_meta(meta),
                    address: encode_document_address(address),
                    chunk_digest: digest_hex(chunk_digest),
                },
            )),
            EmissionContributionWitness::Elided { meta } => {
                wire::EmissionContributionWitness::Elided(Box::new(
                    wire::EmissionContributionWitnessElided {
                        meta: encode_meta(meta),
                    },
                ))
            }
            EmissionContributionWitness::Hoisted { meta, target } => {
                wire::EmissionContributionWitness::Hoisted(Box::new(
                    wire::EmissionContributionWitnessHoisted {
                        meta: encode_meta(meta),
                        target: encode_spec_address(target),
                    },
                ))
            }
        });
    }
    let mut renames = Vec::with_capacity(provenance.renames.len());
    for rename in &provenance.renames {
        renames.push(wire::OriginRename {
            origin: rename.origin.clone(),
            rename: wire::RenameEntry {
                original: rename.rename.original.clone(),
                qualified: rename.rename.qualified.clone(),
            },
        });
    }
    Ok(wire::EmittedArtifact {
        provenance: wire::EmissionProvenance {
            context: encode_context(&provenance.context)?,
            backend: provenance.backend.as_str().to_string(),
            producer: provenance.producer.as_str().to_string(),
            source_lane_digest: digest_hex(&provenance.source_lane_digest.0),
            renames,
            contributions,
            // The one digest independently computable from the value: the
            // manager's own digest of the bytes, recomputed rather than trusted.
            bytes_digest: digest_hex(&emitted_bytes_digest(value.bytes())),
        },
        bytes_b64: encode_base64(value.bytes()),
    })
}
