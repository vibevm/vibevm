use crate::WorkspaceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransactionFailureDisposition {
    Uncommitted,
    RestoredBefore,
    CommitRecoveryIntent,
    RollbackRecoveryIntent,
    EntryRecoveryFailed,
    Indeterminate,
}

#[derive(Debug)]
pub(crate) struct DetailedTransactionFailure {
    source: WorkspaceError,
    disposition: TransactionFailureDisposition,
}

impl DetailedTransactionFailure {
    pub(super) const fn new(
        source: WorkspaceError,
        disposition: TransactionFailureDisposition,
    ) -> Self {
        Self {
            source,
            disposition,
        }
    }

    #[must_use]
    pub(crate) const fn disposition(&self) -> TransactionFailureDisposition {
        self.disposition
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) const fn source_error(&self) -> &WorkspaceError {
        &self.source
    }

    pub(crate) fn into_source(self) -> WorkspaceError {
        self.source
    }
}
