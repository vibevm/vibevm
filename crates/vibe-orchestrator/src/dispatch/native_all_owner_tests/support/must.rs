pub(in crate::dispatch::native_all_owner_tests) type NativeOutputs =
    Vec<(String, Vec<(String, Vec<u8>)>)>;

pub(in crate::dispatch::native_all_owner_tests) fn must<T, E: std::fmt::Debug>(
    value: Result<T, E>,
    context: &str,
) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error:?}"),
    }
}

pub(in crate::dispatch::native_all_owner_tests) fn must_some<T>(
    value: Option<T>,
    context: &str,
) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}
