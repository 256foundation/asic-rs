use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::Runtime;

pub(crate) fn block_on<T>(fut: impl Future<Output = T>) -> Result<T, String> {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    let runtime = match RUNTIME.get() {
        Some(runtime) => runtime,
        None => {
            let created =
                Runtime::new().map_err(|e| format!("failed to create Tokio runtime: {e}"))?;
            RUNTIME.get_or_init(|| created)
        }
    };
    Ok(runtime.block_on(fut))
}
