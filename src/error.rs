/// error type for the server
#[derive(Debug, thiserror::Error)]
pub enum GeneralError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
	#[error("{0}")]
	Custom(String),
	#[allow(unused)]
	#[error("{0}")]
	CustomPrivate(String),
}
