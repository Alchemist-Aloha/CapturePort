use std::sync::OnceLock;

static INITIALIZATION: OnceLock<Result<(), String>> = OnceLock::new();

pub fn initialize() -> Result<(), String> {
    INITIALIZATION
        .get_or_init(|| {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "captureport=info".into()),
                )
                .try_init()
                .map_err(|error| error.to_string())
        })
        .clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn logging_initializes_once() {
        assert!(super::initialize().is_ok());
        assert!(super::initialize().is_ok());
    }
}
