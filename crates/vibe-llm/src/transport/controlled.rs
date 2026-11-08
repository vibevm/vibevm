//! The stop-capable exchange: one owned async request, joined before return.
//!
//! A scoped thread owns a current-thread runtime that drives exactly one
//! request and its capped body read, raced against the operation's stop.
//! When the stop wins, the exchange future is dropped — which closes the
//! connection — and the runtime is dropped with every task it owns before the
//! scope joins the thread. Nothing is detached, nothing outlives the call and
//! nothing is retried. The provider-side effect is recorded on the token: a
//! request that was interrupted, timed out or lost stays `unknown`, because
//! the provider may already have accepted and billed it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use vibe_core::cancellation::CancellationToken;

use super::{MAX_CHAT_RESPONSE_BYTES, TransportError, TransportResponse};

/// How often the stop observation is polled while the request is in flight.
const STOP_POLL: Duration = Duration::from_millis(20);

enum Failure {
    /// The connection was never established: no request bytes left.
    NotDelivered,
    /// The request may have reached the provider.
    Unknown(TransportError),
    /// The provider answered with a status before the exchange failed.
    Answered(TransportError),
}

pub(super) fn exchange(
    client: &reqwest::Client,
    request: reqwest::Request,
    cancellation: &CancellationToken,
) -> Result<TransportResponse, TransportError> {
    let effect = cancellation.llm_request_started();
    let outcome = std::thread::scope(|scope| {
        scope
            .spawn(|| drive(client, request, cancellation))
            .join()
            .unwrap_or(Err(Failure::Unknown(TransportError::RequestFailed)))
    });
    match outcome {
        Ok(response) => {
            effect.completed();
            Ok(response)
        }
        Err(Failure::Answered(error)) => {
            effect.completed();
            Err(error)
        }
        Err(Failure::NotDelivered) => {
            effect.not_delivered();
            Err(TransportError::RequestFailed)
        }
        // Dropping the unresolved handle records the effect as unknown.
        Err(Failure::Unknown(error)) => {
            drop(effect);
            Err(error)
        }
    }
}

fn drive(
    client: &reqwest::Client,
    request: reqwest::Request,
    cancellation: &CancellationToken,
) -> Result<TransportResponse, Failure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Failure::NotDelivered)?;
    let answered = AtomicBool::new(false);
    let result = runtime.block_on(async {
        tokio::select! {
            biased;
            () = stopped(cancellation) => Err(if answered.load(Ordering::SeqCst) {
                Failure::Answered(TransportError::Cancelled)
            } else {
                Failure::Unknown(TransportError::Cancelled)
            }),
            result = send(client, request, &answered) => result,
        }
    });
    // Dropping the runtime drops the connection tasks the request spawned.
    drop(runtime);
    result
}

async fn stopped(cancellation: &CancellationToken) {
    while cancellation.checkpoint().is_ok() {
        tokio::time::sleep(STOP_POLL).await;
    }
}

async fn send(
    client: &reqwest::Client,
    request: reqwest::Request,
    answered: &AtomicBool,
) -> Result<TransportResponse, Failure> {
    let mut response = client.execute(request).await.map_err(|error| {
        if error.is_connect() {
            Failure::NotDelivered
        } else {
            Failure::Unknown(TransportError::RequestFailed)
        }
    })?;
    answered.store(true, Ordering::SeqCst);
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Ok(TransportResponse::new(status, None, Vec::new()));
    }
    let mut body = Vec::with_capacity(16 * 1024);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Failure::Answered(TransportError::ResponseReadFailed))?
    {
        if body.len() + chunk.len() > MAX_CHAT_RESPONSE_BYTES {
            return Err(Failure::Answered(TransportError::ResponseTooLarge {
                limit: MAX_CHAT_RESPONSE_BYTES,
            }));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(TransportResponse::new(status, None, body))
}

#[cfg(test)]
mod tests;
