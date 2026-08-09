//! Typed IPC protocol: request/response contract, service identity, and the
//! error model.
//!
//! The wire format is JSON (via `serde_json`), consistent with the rest of
//! the foundation. The protocol is transport-agnostic; see
//! [`super::transport`].

use pursue_core::{Error, Result};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use std::fmt;

/// Maximum length (in bytes) of a service identifier or method name.
pub const MAX_NAME_LEN: usize = 64;

/// Validates a service/method identifier: 1–64 characters, lowercase ASCII
/// alphanumeric plus `-`, `_`, `.`, starting with an ASCII alphanumeric.
fn validate_name(kind: &str, name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::InvalidInput(format!("{kind} must not be empty")));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(Error::InvalidInput(format!(
            "{kind} exceeds {MAX_NAME_LEN} characters"
        )));
    }
    let mut chars = name.chars();
    let first = chars.next().expect("non-empty checked above");
    if !first.is_ascii_alphanumeric() {
        return Err(Error::InvalidInput(format!(
            "{kind} must start with an ASCII alphanumeric character: {name:?}"
        )));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        return Err(Error::InvalidInput(format!(
            "{kind} may only contain lowercase ASCII alphanumerics, '-', '_', '.'"
        )));
    }
    if name.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(Error::InvalidInput(format!(
            "{kind} must be lowercase: {name:?}"
        )));
    }
    Ok(name.to_string())
}

/// The stable identity of a service on the IPC boundary.
///
/// Validated on construction *and* on deserialization, so malformed wire
/// identifiers can never reach a router.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceId(String);

impl ServiceId {
    /// Creates an identifier, validating the name rules.
    pub fn new(name: &str) -> Result<Self> {
        Ok(Self(validate_name("service id", name)?))
    }

    /// The identifier as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for ServiceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ServiceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ServiceId::new(&s).map_err(D::Error::custom)
    }
}

/// The name of a method exposed by a service.
///
/// Uses the same identifier rules as [`ServiceId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodName(ServiceId);

impl MethodName {
    /// Creates a method name, validating the identifier rules.
    pub fn new(name: &str) -> Result<Self> {
        Ok(Self(ServiceId::new(name)?))
    }

    /// The method name as a string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MethodName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for MethodName {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MethodName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let inner = ServiceId::deserialize(deserializer)?;
        Ok(Self(inner))
    }
}

/// A typed IPC request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Caller-supplied identifier, echoed back in the response.
    pub id: u64,
    /// The target service.
    pub service: ServiceId,
    /// The method to invoke on the service.
    pub method: MethodName,
    /// JSON parameters; each service defines its own parameter schema.
    pub params: JsonValue,
}

impl Request {
    /// Creates a request. `service` and `method` must already be valid.
    pub fn new(id: u64, service: ServiceId, method: MethodName, params: JsonValue) -> Self {
        Self {
            id,
            service,
            method,
            params,
        }
    }
}

/// Stable machine-readable error codes carried on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    /// The message itself was malformed.
    InvalidRequest,
    /// The requested service is not registered.
    UnknownService,
    /// The service does not expose the requested method.
    UnknownMethod,
    /// The parameters did not satisfy the method contract.
    InvalidParams,
    /// The caller lacks the capability required for this operation.
    PermissionDenied,
    /// The service failed internally.
    Internal,
    /// The service is not currently available.
    ServiceUnavailable,
}

impl IpcErrorCode {
    /// The stable wire name of this code.
    pub const fn as_str(self) -> &'static str {
        match self {
            IpcErrorCode::InvalidRequest => "invalid_request",
            IpcErrorCode::UnknownService => "unknown_service",
            IpcErrorCode::UnknownMethod => "unknown_method",
            IpcErrorCode::InvalidParams => "invalid_params",
            IpcErrorCode::PermissionDenied => "permission_denied",
            IpcErrorCode::Internal => "internal",
            IpcErrorCode::ServiceUnavailable => "service_unavailable",
        }
    }
}

impl fmt::Display for IpcErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed IPC failure with a stable code and a human-readable message.
///
/// Messages are diagnostics only; they must never contain evidence content,
/// secrets, or credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IpcError {
    code: IpcErrorCode,
    message: String,
}

/// Intermediate representation used to validate deserialized errors.
#[derive(Deserialize)]
struct IpcErrorRepr {
    code: IpcErrorCode,
    message: String,
}

impl IpcError {
    /// Creates a failure; `message` must be non-empty after trimming.
    pub fn new(code: IpcErrorCode, message: &str) -> Result<Self> {
        let message = message.trim();
        if message.is_empty() {
            return Err(Error::InvalidInput(
                "ipc error message must not be empty".into(),
            ));
        }
        Ok(Self {
            code,
            message: message.to_string(),
        })
    }

    /// The stable error code.
    pub fn code(&self) -> IpcErrorCode {
        self.code
    }

    /// The human-readable diagnostic message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for IpcError {}

impl<'de> Deserialize<'de> for IpcError {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let repr = IpcErrorRepr::deserialize(deserializer)?;
        IpcError::new(repr.code, &repr.message).map_err(D::Error::custom)
    }
}

/// A typed IPC response: exactly one of `result` or `error` is set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Echo of the request identifier.
    pub id: u64,
    /// Successful result payload, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result: Option<JsonValue>,
    /// Failure description, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<IpcError>,
}

impl Response {
    /// A successful response carrying `result`.
    pub fn success(id: u64, result: JsonValue) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    /// A failed response carrying `error`.
    pub fn failure(id: u64, error: IpcError) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Whether this response represents success.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Validates the exactly-one-of `result`/`error` invariant.
    pub fn validate(&self) -> Result<()> {
        match (&self.result, &self.error) {
            (Some(_), Some(_)) => Err(Error::InvalidInput(
                "response has both result and error".into(),
            )),
            (None, None) => Err(Error::InvalidInput(
                "response has neither result nor error".into(),
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IpcError, IpcErrorCode, MAX_NAME_LEN, MethodName, Request, Response, ServiceId};
    use serde_json::{Value, json};

    #[test]
    fn valid_identifiers_are_accepted() {
        for name in ["core", "evidence-store", "report.export", "a_1", "x-y"] {
            ServiceId::new(name).unwrap();
            MethodName::new(name).unwrap();
        }
    }

    #[test]
    fn invalid_identifiers_are_rejected() {
        for name in [
            "",
            "   ",
            "Upper",
            "UPPER",
            "-leading",
            "_leading",
            "has space",
            "bad/name",
            "bad@name",
            &"x".repeat(MAX_NAME_LEN + 1),
        ] {
            assert!(ServiceId::new(name).is_err(), "should reject {name:?}");
            assert!(MethodName::new(name).is_err(), "should reject {name:?}");
        }
    }

    #[test]
    fn service_id_serde_roundtrip() {
        let id = ServiceId::new("evidence-store").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"evidence-store\"");
        let back: ServiceId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn malformed_service_id_on_wire_is_rejected() {
        for wire in ["\"\"", "\"Bad\"", "42", "\"with space\""] {
            assert!(
                serde_json::from_str::<ServiceId>(wire).is_err(),
                "should reject {wire}"
            );
            assert!(
                serde_json::from_str::<MethodName>(wire).is_err(),
                "should reject {wire}"
            );
        }
    }

    #[test]
    fn request_roundtrips() {
        let request = Request::new(
            7,
            ServiceId::new("echo").unwrap(),
            MethodName::new("ping").unwrap(),
            json!({"hello": "world"}),
        );
        let json = serde_json::to_string(&request).unwrap();
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, request);
    }

    #[test]
    fn malformed_request_is_rejected() {
        assert!(serde_json::from_str::<Request>("{}").is_err());
        assert!(serde_json::from_str::<Request>(r#"{"id":1}"#).is_err());
        assert!(
            serde_json::from_str::<Request>(r#"{"id":1,"service":"echo","method":"ping"}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<Request>(
                r#"{"id":1,"service":"Echo","method":"ping","params":{}}"#
            )
            .is_err()
        );
    }

    #[test]
    fn success_response_shape() {
        let response = Response::success(3, json!({"ok": true}));
        response.validate().unwrap();
        assert!(response.is_success());
        let value: Value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["id"], json!(3));
        assert_eq!(value["result"], json!({"ok": true}));
        assert!(value.get("error").is_none());
    }

    #[test]
    fn failure_response_shape() {
        let error = IpcError::new(IpcErrorCode::UnknownMethod, "no such method").unwrap();
        let response = Response::failure(4, error);
        response.validate().unwrap();
        assert!(!response.is_success());
        let value: Value = serde_json::to_value(&response).unwrap();
        assert!(value.get("result").is_none());
        assert_eq!(value["error"]["code"], json!("unknown_method"));
        assert_eq!(value["error"]["message"], json!("no such method"));
    }

    #[test]
    fn response_validate_rejects_both_or_neither() {
        let both = Response {
            id: 1,
            result: Some(json!(1)),
            error: Some(IpcError::new(IpcErrorCode::Internal, "boom").unwrap()),
        };
        assert!(both.validate().is_err());
        let neither = Response {
            id: 1,
            result: None,
            error: None,
        };
        assert!(neither.validate().is_err());
    }

    #[test]
    fn ipc_error_roundtrip_and_validation() {
        let error = IpcError::new(IpcErrorCode::PermissionDenied, "denied").unwrap();
        assert_eq!(error.code(), IpcErrorCode::PermissionDenied);
        assert_eq!(error.message(), "denied");
        assert_eq!(error.to_string(), "permission_denied: denied");

        let json = serde_json::to_string(&error).unwrap();
        let back: IpcError = serde_json::from_str(&json).unwrap();
        assert_eq!(back, error);
    }

    #[test]
    fn malformed_error_is_rejected() {
        // Empty message.
        assert!(serde_json::from_str::<IpcError>(r#"{"code":"internal","message":""}"#).is_err());
        // Unknown code.
        assert!(serde_json::from_str::<IpcError>(r#"{"code":"mystery","message":"x"}"#).is_err());
        // Empty constructor input.
        assert!(IpcError::new(IpcErrorCode::Internal, "  ").is_err());
    }

    #[test]
    fn error_code_names_are_stable() {
        assert_eq!(IpcErrorCode::InvalidRequest.as_str(), "invalid_request");
        assert_eq!(IpcErrorCode::UnknownService.as_str(), "unknown_service");
        assert_eq!(IpcErrorCode::UnknownMethod.as_str(), "unknown_method");
        assert_eq!(IpcErrorCode::InvalidParams.as_str(), "invalid_params");
        assert_eq!(IpcErrorCode::PermissionDenied.as_str(), "permission_denied");
        assert_eq!(IpcErrorCode::Internal.as_str(), "internal");
        assert_eq!(
            IpcErrorCode::ServiceUnavailable.as_str(),
            "service_unavailable"
        );
    }
}
